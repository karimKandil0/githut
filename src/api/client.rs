use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use octocrab::Octocrab;
use serde::Deserialize;

use crate::types::{EntryType, FileEntry, RateLimit, Repo, SearchResult};

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

        // fallback: try download_url
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
}
