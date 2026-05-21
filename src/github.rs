use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Deserialize)]
struct RepoSearchResponse {
    items: Vec<RepoItem>,
}

#[derive(Deserialize)]
struct RepoItem {
    full_name: String,
}

#[derive(Deserialize)]
struct WatchedRepo {
    full_name: String,
}

#[derive(Deserialize)]
struct PullRequest {
    html_url: String,
}

#[derive(Deserialize)]
struct PrSearchResponse {
    items: Vec<PrSearchItem>,
}

#[derive(Deserialize)]
struct PrSearchItem {
    number: u32,
    title: String,
    user: PrUser,
    repository_url: String,
    created_at: String,
    draft: Option<bool>,
    html_url: String,
}

#[derive(Deserialize)]
struct PrUser {
    login: String,
}

#[derive(Clone)]
pub struct PrItem {
    pub number: u32,
    pub title: String,
    pub author: String,
    pub repo: String,
    pub created_at: String,
    pub draft: bool,
    pub html_url: String,
}

#[derive(Deserialize)]
struct ApiCommit {
    sha: String,
    commit: ApiCommitDetail,
}

#[derive(Deserialize)]
struct ApiCommitDetail {
    author: ApiCommitAuthor,
    message: String,
}

#[derive(Deserialize)]
struct ApiCommitAuthor {
    name: String,
    date: String,
}

#[derive(Clone)]
pub struct CommitInfo {
    pub sha: String,
    pub author: String,
    pub message: String,
    pub date: String,
}

#[derive(Clone)]
pub struct RepoPreview {
    pub languages: Vec<(String, f64)>,
    pub commits: Vec<CommitInfo>,
}

pub struct GithubClient {
    client: Client,
    token: String,
    pub orgs: Vec<String>,
    base_url: String,
}

impl GithubClient {
    pub fn new(token: String, orgs: Vec<String>) -> Result<Self> {
        let client = Client::builder().user_agent("acv-terminal/0.1").build()?;
        let base_url = std::env::var("NORSE_GITHUB_API")
            .unwrap_or_else(|_| "https://api.github.com".to_string());
        Ok(Self { client, token, orgs, base_url })
    }

    pub async fn search_repos(&self, query: &str) -> Result<Vec<String>> {
        let mut seen = HashSet::new();
        let mut repos = Vec::new();
        for org in &self.orgs {
            for repo in self.search_org(query, org).await.unwrap_or_default() {
                if seen.insert(repo.clone()) {
                    repos.push(repo);
                }
            }
        }
        Ok(repos)
    }

    async fn search_org(&self, query: &str, org: &str) -> Result<Vec<String>> {
        let q = format!("{} in:name org:{} archived:false", query, org);
        let resp: RepoSearchResponse = self
            .client
            .get(format!("{}/search/repositories", self.base_url))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .query(&[("q", q.as_str()), ("per_page", "30")])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(resp.items.into_iter().map(|i| i.full_name).collect())
    }


    pub async fn search_reviews_requested(&self) -> Result<Vec<PrItem>> {
        let q = "is:pr is:open review-requested:@me archived:false";
        let resp: PrSearchResponse = self
            .client
            .get(format!("{}/search/issues", self.base_url))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .query(&[("q", q), ("sort", "created"), ("per_page", "50")])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let repos_prefix = format!("{}/repos/", self.base_url);
        let mut prs: Vec<PrItem> = resp.items.into_iter().map(|i| {
            let repo = i.repository_url
                .strip_prefix(&repos_prefix)
                .unwrap_or(&i.repository_url)
                .to_string();
            PrItem {
                number: i.number,
                title: i.title,
                author: i.user.login,
                repo,
                created_at: i.created_at.clone(),
                draft: i.draft.unwrap_or(false),
                html_url: i.html_url,
            }
        }).collect();
        prs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(prs)
    }

    pub async fn search_prs(&self) -> Result<Vec<PrItem>> {
        let mut prs = Vec::new();
        for org in &self.orgs {
            prs.extend(self.search_org_prs(org).await.unwrap_or_default());
        }
        prs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        prs.truncate(100);
        Ok(prs)
    }

    async fn search_org_prs(&self, org: &str) -> Result<Vec<PrItem>> {
        let q = format!("is:pr is:open org:{} archived:false", org);
        let resp: PrSearchResponse = self
            .client
            .get(format!("{}/search/issues", self.base_url))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .query(&[("q", q.as_str()), ("sort", "updated"), ("per_page", "50")])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let repos_prefix = format!("{}/repos/", self.base_url);
        Ok(resp.items.into_iter().map(|i| {
            let repo = i.repository_url
                .strip_prefix(&repos_prefix)
                .unwrap_or(&i.repository_url)
                .to_string();
            PrItem {
                number: i.number,
                title: i.title,
                author: i.user.login,
                repo,
                created_at: i.created_at.clone(),

                draft: i.draft.unwrap_or(false),
                html_url: i.html_url,
            }
        }).collect())
    }

    // Returns the HTML URL of the first PR containing this commit, or an error
    // if no PR exists (caller can fall back to the commit URL).
    pub async fn get_pr_url(&self, repo: &str, sha: &str) -> Result<String> {
        let prs: Vec<PullRequest> = self
            .client
            .get(format!("{}/repos/{}/commits/{}/pulls", self.base_url, repo, sha))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        prs.into_iter()
            .next()
            .map(|pr| pr.html_url)
            .ok_or_else(|| anyhow::anyhow!("no PR found for this commit"))
    }

    pub async fn get_repo_preview(&self, repo: &str) -> Result<RepoPreview> {
        let (languages, commits) = tokio::try_join!(
            self.fetch_languages(repo),
            self.fetch_commits(repo),
        )?;
        Ok(RepoPreview { languages, commits })
    }

    async fn fetch_languages(&self, repo: &str) -> Result<Vec<(String, f64)>> {
        let map: HashMap<String, u64> = self
            .client
            .get(format!("{}/repos/{}/languages", self.base_url, repo))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let total: u64 = map.values().sum();
        if total == 0 {
            return Ok(vec![]);
        }

        let mut langs: Vec<(String, f64)> = map
            .into_iter()
            .map(|(k, v)| (k, v as f64 / total as f64 * 100.0))
            .collect();
        langs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(langs)
    }

    pub async fn get_pr_diff(&self, repo: &str, number: u32) -> Result<String> {
        let text = self
            .client
            .get(format!("{}/repos/{}/pulls/{}", self.base_url, repo, number))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.diff")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(text)
    }

    pub async fn get_commit_diff(&self, repo: &str, sha: &str) -> Result<String> {
        let text = self
            .client
            .get(format!("{}/repos/{}/commits/{}", self.base_url, repo, sha))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.diff")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(text)
    }

    async fn fetch_commits(&self, repo: &str) -> Result<Vec<CommitInfo>> {
        let commits: Vec<ApiCommit> = self
            .client
            .get(format!("{}/repos/{}/commits", self.base_url, repo))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .query(&[("per_page", "5")])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(commits
            .into_iter()
            .map(|c| CommitInfo {
                sha: c.sha[..7.min(c.sha.len())].to_string(),
                author: c.commit.author.name,
                message: c.commit.message.lines().next().unwrap_or("").to_string(),
                date: c.commit.author.date[..10.min(c.commit.author.date.len())].to_string(),
            })
            .collect())
    }

    pub async fn fetch_watched_repos(&self) -> Result<HashSet<String>> {
        let mut page = 1u32;
        let mut all = HashSet::new();
        loop {
            let repos: Vec<WatchedRepo> = self
                .client
                .get(format!("{}/user/subscriptions", self.base_url))
                .header("Authorization", format!("Bearer {}", self.token))
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .query(&[("per_page", "100"), ("page", &page.to_string())])
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let done = repos.len() < 100;
            for r in repos {
                all.insert(r.full_name);
            }
            if done { break; }
            page += 1;
        }
        Ok(all)
    }

    pub async fn watch_repo(&self, owner: &str, repo: &str) -> Result<()> {
        self.client
            .put(format!("{}/repos/{}/{}/subscription", self.base_url, owner, repo))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(&serde_json::json!({"subscribed": true, "ignored": false}))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn unwatch_repo(&self, owner: &str, repo: &str) -> Result<()> {
        self.client
            .delete(format!("{}/repos/{}/{}/subscription", self.base_url, owner, repo))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
