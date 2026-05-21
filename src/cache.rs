use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Clone)]
pub struct PrCache {
    pub pr_number: u32,
    pub timestamp: u64,
    pub summary: String,
}

fn pr_path(repo: &str, pr_number: u32) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let (owner, name) = repo.split_once('/')?;
    Some(PathBuf::from(format!(
        "{}/.norsedata/repos/{}/{}/pr-{}.toml",
        home, owner, name, pr_number
    )))
}

fn repo_dir(repo: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let (owner, name) = repo.split_once('/')?;
    Some(PathBuf::from(format!(
        "{}/.norsedata/repos/{}/{}",
        home, owner, name
    )))
}

pub fn load_pr_cache(repo: &str, pr_number: u32) -> Option<PrCache> {
    let text = std::fs::read_to_string(pr_path(repo, pr_number)?).ok()?;
    toml::from_str(&text).ok()
}

pub fn save_pr_cache(repo: &str, pr_number: u32, summary: &str) {
    let Some(path) = pr_path(repo, pr_number) else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let cache = PrCache { pr_number, timestamp, summary: summary.to_string() };
    if let Ok(text) = toml::to_string(&cache) {
        let _ = std::fs::write(path, text);
    }
}

// Returns a context string built from the N most recent cached PR summaries for this repo,
// excluding the current PR so we don't feed a stale version of itself back in.
pub fn load_repo_context(repo: &str, exclude_pr: u32) -> Option<String> {
    let dir = repo_dir(repo)?;
    let mut entries: Vec<PrCache> = std::fs::read_dir(&dir).ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("pr-") || !name.ends_with(".toml") { return None; }
            let text = std::fs::read_to_string(e.path()).ok()?;
            toml::from_str::<PrCache>(&text).ok()
        })
        .filter(|c| c.pr_number != exclude_pr)
        .collect();

    if entries.is_empty() { return None; }

    // Most recent first, cap at 5
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    entries.truncate(5);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut out = String::from("Recent PR context for this repository:\n");
    for c in &entries {
        let age_days = (now.saturating_sub(c.timestamp)) / 86400;
        let age = if age_days == 0 { "today".into() } else { format!("{} days ago", age_days) };
        out.push_str(&format!("\n--- PR #{} ({}) ---\n{}\n", c.pr_number, age, c.summary));
    }
    Some(out)
}
