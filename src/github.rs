use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use octocrab::Octocrab;
use serde::Deserialize;

use crate::types::{Repo, SearchResult};

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
