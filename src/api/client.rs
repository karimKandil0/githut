use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use octocrab::Octocrab;
use serde::Deserialize;

use crate::types::{
    EntryType, FileEntry, Issue, IssueComment, IssueFilter, Notification, RateLimit, Repo,
    SearchResult, UserProfile,
};

pub struct GithubClient {
    inner: Octocrab,
}

#[derive(Deserialize)]
struct SearchResponse {
    total_count: u64,
    items: Vec<RepoItem>,
}

#[derive(Deserialize)]
struct RepoItem {
    id: u64,
    full_name: String,
    name: String,
    owner: OwnerItem,
    description: Option<String>,
    language: Option<String>,
    stargazers_count: u64,
    forks_count: u64,
    html_url: String,
    clone_url: String,
    default_branch: String,
    #[serde(default)]
    archived: bool,
    #[serde(default)]
    fork: bool,
}

#[derive(Deserialize)]
struct OwnerItem {
    login: String,
}

#[derive(Deserialize)]
struct ReadmeResponse {
    content: String,
    encoding: String,
}

#[derive(Deserialize)]
struct ContentItem {
    name: String,
    path: String,
    #[serde(rename = "type")]
    item_type: String,
    download_url: Option<String>,
    content: Option<String>,
    encoding: Option<String>,
}

impl GithubClient {
    pub fn new(token: &str) -> Result<Self> {
        let inner = Octocrab::builder()
            .personal_token(token.to_string())
            .build()
            .context("failed to build octocrab client")?;
        Ok(Self { inner })
    }

    pub async fn search_repos(&self, query: &str, language: Option<&str>) -> Result<SearchResult> {
        let q = match language {
            Some(lang) => format!("{} language:{}", query, lang),
            None => query.to_string(),
        };

        let response: SearchResponse = self
            .inner
            .get(
                format!(
                    "/search/repositories?q={}&per_page=30&sort=stars",
                    urlencoding::encode(&q)
                ),
                None::<&()>,
            )
            .await
            .context("search request failed")?;

        let repos = response
            .items
            .into_iter()
            .map(|item| Repo {
                id: item.id,
                full_name: item.full_name,
                name: item.name,
                owner: item.owner.login,
                description: item.description,
                language: item.language,
                stargazers_count: item.stargazers_count,
                forks_count: item.forks_count,
                html_url: item.html_url,
                clone_url: item.clone_url,
                default_branch: item.default_branch,
                archived: item.archived,
                fork: item.fork,
            })
            .collect();

        Ok(SearchResult {
            repos,
            total_count: response.total_count,
        })
    }

    pub async fn get_contents(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
    ) -> Result<Vec<FileEntry>> {
        let url = if path.is_empty() {
            format!("/repos/{}/{}/contents", owner, repo)
        } else {
            format!("/repos/{}/{}/contents/{}", owner, repo, path)
        };

        let items: Vec<ContentItem> = self
            .inner
            .get(url, None::<&()>)
            .await
            .context("contents request failed")?;

        let mut entries: Vec<FileEntry> = items
            .into_iter()
            .map(|item| FileEntry {
                name: item.name,
                path: item.path,
                entry_type: if item.item_type == "dir" {
                    EntryType::Dir
                } else {
                    EntryType::File
                },
            })
            .collect();

        // dirs first, then files, both alphabetical
        entries.sort_by(|a, b| match (&a.entry_type, &b.entry_type) {
            (EntryType::Dir, EntryType::File) => std::cmp::Ordering::Less,
            (EntryType::File, EntryType::Dir) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        Ok(entries)
    }

    pub async fn get_file_content(&self, owner: &str, repo: &str, path: &str) -> Result<String> {
        let url = format!("/repos/{}/{}/contents/{}", owner, repo, path);

        let item: ContentItem = self
            .inner
            .get(url, None::<&()>)
            .await
            .context("file content request failed")?;

        if let (Some(content), Some(encoding)) = (item.content, item.encoding) {
            if encoding == "base64" {
                let clean = content.replace('\n', "");
                let bytes = general_purpose::STANDARD
                    .decode(&clean)
                    .context("failed to base64-decode file content")?;
                return Ok(String::from_utf8_lossy(&bytes).to_string());
            }
        }

        // fallback for large files: fetch raw content via download_url
        if let Some(url) = item.download_url {
            let text = reqwest::get(&url)
                .await
                .context("download_url fetch failed")?
                .text()
                .await
                .context("download_url read failed")?;
            return Ok(text);
        }

        Err(anyhow!("could not decode file content"))
    }

    pub async fn is_starred(&self, owner: &str, repo: &str) -> bool {
        self.inner
            .get::<serde_json::Value, _, _>(
                format!("/user/starred/{}/{}", owner, repo),
                None::<&()>,
            )
            .await
            .is_ok()
    }

    pub async fn star(&self, owner: &str, repo: &str) -> Result<()> {
        self.inner
            ._put(format!("/user/starred/{}/{}", owner, repo), None::<&()>)
            .await
            .context("star request failed")?;
        Ok(())
    }

    pub async fn unstar(&self, owner: &str, repo: &str) -> Result<()> {
        self.inner
            ._delete(format!("/user/starred/{}/{}", owner, repo), None::<&()>)
            .await
            .context("unstar request failed")?;
        Ok(())
    }

    pub async fn fork(&self, owner: &str, repo: &str) -> Result<()> {
        self.inner
            ._post(
                format!("/repos/{}/{}/forks", owner, repo),
                Some(&serde_json::json!({})),
            )
            .await
            .context("fork request failed")?;
        Ok(())
    }

    pub async fn list_my_repos(&self) -> Result<Vec<Repo>> {
        let items: Vec<RepoItem> = self
            .inner
            .get(
                "/user/repos?sort=updated&per_page=100&affiliation=owner",
                None::<&()>,
            )
            .await
            .context("list repos request failed")?;

        Ok(items
            .into_iter()
            .map(|item| Repo {
                id: item.id,
                full_name: item.full_name,
                name: item.name,
                owner: item.owner.login,
                description: item.description,
                language: item.language,
                stargazers_count: item.stargazers_count,
                forks_count: item.forks_count,
                html_url: item.html_url,
                clone_url: item.clone_url,
                default_branch: item.default_branch,
                archived: item.archived,
                fork: item.fork,
            })
            .collect())
    }

    pub async fn delete_repo(&self, owner: &str, repo: &str) -> Result<()> {
        self.inner
            ._delete(format!("/repos/{}/{}", owner, repo), None::<&()>)
            .await
            .context("delete repo request failed")?;
        Ok(())
    }

    pub async fn rename_repo(&self, owner: &str, repo: &str, new_name: &str) -> Result<()> {
        self.inner
            ._patch(
                format!("/repos/{}/{}", owner, repo),
                Some(&serde_json::json!({ "name": new_name })),
            )
            .await
            .context("rename repo request failed")?;
        Ok(())
    }

    pub async fn set_archived(&self, owner: &str, repo: &str, archived: bool) -> Result<()> {
        self.inner
            ._patch(
                format!("/repos/{}/{}", owner, repo),
                Some(&serde_json::json!({ "archived": archived })),
            )
            .await
            .context("set archived request failed")?;
        Ok(())
    }

    pub async fn get_user_profile(&self, login: &str) -> Result<UserProfile> {
        #[derive(Deserialize)]
        struct ProfileResponse {
            login: String,
            name: Option<String>,
            bio: Option<String>,
            followers: u64,
            following: u64,
            public_repos: u64,
            html_url: String,
        }
        let resp: ProfileResponse = self
            .inner
            .get(format!("/users/{}", login), None::<&()>)
            .await
            .context("user profile request failed")?;
        Ok(UserProfile {
            login: resp.login,
            name: resp.name,
            bio: resp.bio,
            followers: resp.followers,
            following: resp.following,
            public_repos: resp.public_repos,
            html_url: resp.html_url,
        })
    }

    pub async fn list_user_repos(&self, login: &str) -> Result<Vec<Repo>> {
        let items: Vec<RepoItem> = self
            .inner
            .get(
                format!(
                    "/users/{}/repos?sort=updated&per_page=100&type=owner",
                    login
                ),
                None::<&()>,
            )
            .await
            .context("list user repos request failed")?;
        Ok(items
            .into_iter()
            .map(|item| Repo {
                id: item.id,
                full_name: item.full_name,
                name: item.name,
                owner: item.owner.login,
                description: item.description,
                language: item.language,
                stargazers_count: item.stargazers_count,
                forks_count: item.forks_count,
                html_url: item.html_url,
                clone_url: item.clone_url,
                default_branch: item.default_branch,
                archived: item.archived,
                fork: item.fork,
            })
            .collect())
    }

    pub async fn get_rate_limit(&self) -> Result<RateLimit> {
        #[derive(Deserialize)]
        struct RateLimitResponse {
            resources: Resources,
        }
        #[derive(Deserialize)]
        struct Resources {
            search: RateLimitItem,
            core: RateLimitItem,
        }
        #[derive(Deserialize)]
        struct RateLimitItem {
            remaining: u32,
            limit: u32,
        }

        let resp: RateLimitResponse = self
            .inner
            .get("/rate_limit", None::<&()>)
            .await
            .context("rate limit request failed")?;

        Ok(RateLimit {
            search_remaining: resp.resources.search.remaining,
            search_limit: resp.resources.search.limit,
            core_remaining: resp.resources.core.remaining,
            core_limit: resp.resources.core.limit,
        })
    }

    pub async fn get_readme(&self, owner: &str, repo: &str) -> Result<String> {
        let response: ReadmeResponse = self
            .inner
            .get(format!("/repos/{}/{}/readme", owner, repo), None::<&()>)
            .await
            .context("readme request failed")?;

        if response.encoding != "base64" {
            return Err(anyhow!("unexpected readme encoding: {}", response.encoding));
        }

        let clean = response.content.replace('\n', "");
        let bytes = general_purpose::STANDARD
            .decode(&clean)
            .context("failed to base64-decode readme")?;
        let text = String::from_utf8_lossy(&bytes).to_string();
        Ok(text)
    }

    pub async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        filter: &IssueFilter,
        is_pr: bool,
    ) -> Result<Vec<Issue>> {
        #[derive(Deserialize)]
        struct LabelItem {
            name: String,
        }
        #[derive(Deserialize)]
        struct IssueItem {
            number: u64,
            title: String,
            state: String,
            user: UserLogin,
            body: Option<String>,
            comments: u64,
            created_at: String,
            html_url: String,
            pull_request: Option<serde_json::Value>,
            labels: Vec<LabelItem>,
        }
        #[derive(Deserialize)]
        struct UserLogin {
            login: String,
        }

        let type_filter = if is_pr { "pulls" } else { "issues" };
        let url = format!(
            "/repos/{}/{}/issues?state={}&per_page=50&sort=updated&type={}",
            owner,
            repo,
            filter.as_str(),
            type_filter
        );
        // GitHub issues endpoint returns both issues and PRs; filter by pull_request field
        let items: Vec<IssueItem> = self
            .inner
            .get(url, None::<&()>)
            .await
            .context("list issues request failed")?;

        Ok(items
            .into_iter()
            .filter(|item| {
                let has_pr = item.pull_request.is_some();
                has_pr == is_pr
            })
            .map(|item| Issue {
                number: item.number,
                title: item.title,
                state: item.state,
                user_login: item.user.login,
                body: item.body,
                comments: item.comments,
                created_at: item.created_at,
                html_url: item.html_url,
                pull_request: item.pull_request.is_some(),
                labels: item.labels.into_iter().map(|l| l.name).collect(),
            })
            .collect())
    }

    pub async fn get_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Result<Vec<IssueComment>> {
        #[derive(Deserialize)]
        struct CommentItem {
            id: u64,
            user: UserLogin,
            body: String,
            created_at: String,
        }
        #[derive(Deserialize)]
        struct UserLogin {
            login: String,
        }

        let url = format!(
            "/repos/{}/{}/issues/{}/comments?per_page=50",
            owner, repo, number
        );
        let items: Vec<CommentItem> = self
            .inner
            .get(url, None::<&()>)
            .await
            .context("get issue comments failed")?;

        Ok(items
            .into_iter()
            .map(|c| IssueComment {
                id: c.id,
                user_login: c.user.login,
                body: c.body,
                created_at: c.created_at,
            })
            .collect())
    }

    pub async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
    ) -> Result<u64> {
        #[derive(Deserialize)]
        struct CreatedIssue {
            number: u64,
        }
        let resp: CreatedIssue = self
            .inner
            .post(
                format!("/repos/{}/{}/issues", owner, repo),
                Some(&serde_json::json!({ "title": title, "body": body })),
            )
            .await
            .context("create issue failed")?;
        Ok(resp.number)
    }

    pub async fn close_issue(&self, owner: &str, repo: &str, number: u64) -> Result<()> {
        self.inner
            ._patch(
                format!("/repos/{}/{}/issues/{}", owner, repo, number),
                Some(&serde_json::json!({ "state": "closed" })),
            )
            .await
            .context("close issue failed")?;
        Ok(())
    }

    pub async fn list_notifications(&self, only_unread: bool) -> Result<Vec<Notification>> {
        #[derive(Deserialize)]
        struct NotifItem {
            id: String,
            repository: NotifRepo,
            subject: NotifSubject,
            reason: String,
            unread: bool,
            updated_at: String,
        }
        #[derive(Deserialize)]
        struct NotifRepo {
            full_name: String,
        }
        #[derive(Deserialize)]
        struct NotifSubject {
            title: String,
            #[serde(rename = "type")]
            subject_type: String,
            url: Option<String>,
        }

        let all_param = if only_unread { "false" } else { "true" };
        let url = format!("/notifications?all={}&per_page=50", all_param);
        let items: Vec<NotifItem> = self
            .inner
            .get(url, None::<&()>)
            .await
            .context("list notifications failed")?;

        Ok(items
            .into_iter()
            .map(|n| Notification {
                id: n.id,
                repo_full_name: n.repository.full_name,
                subject_title: n.subject.title,
                subject_type: n.subject.subject_type,
                reason: n.reason,
                unread: n.unread,
                updated_at: n.updated_at,
                subject_url: n.subject.url,
            })
            .collect())
    }

    pub async fn mark_notification_read(&self, id: &str) -> Result<()> {
        self.inner
            ._patch(format!("/notifications/threads/{}", id), None::<&()>)
            .await
            .context("mark notification read failed")?;
        Ok(())
    }

    pub async fn mark_all_notifications_read(&self) -> Result<()> {
        self.inner
            ._put("/notifications", Some(&serde_json::json!({})))
            .await
            .context("mark all notifications read failed")?;
        Ok(())
    }
}
