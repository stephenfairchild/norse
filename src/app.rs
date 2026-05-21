use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

fn norsedata_size() -> Option<u64> {
    let home = std::env::var("HOME").ok()?;
    let path = std::path::Path::new(&home).join(".norsedata");
    dir_size(&path).ok()
}

fn dir_size(path: &std::path::Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_dir() {
            total += dir_size(&entry.path())?;
        } else {
            total += meta.len();
        }
    }
    Ok(total)
}
use crate::config::Config;
use crate::cache::{load_pr_cache, load_repo_context, save_pr_cache};
use crate::github::{extract_jira, GithubClient, PrComment, PrItem, RepoActivity, RepoPreview};
use crate::llm::LlmClient;
use crate::search::{SearchResult, SearchState};

pub enum Mode {
    Normal,
    Picker,
    Diff,
    PrBrowser,
    News,
    ModelPicker,
}

pub enum Focus {
    Results,
    Preview,
    Answer,
}

pub enum PrTab {
    All,
    ByPeople,
    ByRepo,
    ReviewsRequested,
    Watching,
    RecentlyClosed,
}

impl PrTab {
    pub fn next(&self) -> Self {
        match self {
            PrTab::All               => PrTab::ByPeople,
            PrTab::ByPeople          => PrTab::ByRepo,
            PrTab::ByRepo            => PrTab::ReviewsRequested,
            PrTab::ReviewsRequested  => PrTab::Watching,
            PrTab::Watching          => PrTab::RecentlyClosed,
            PrTab::RecentlyClosed    => PrTab::RecentlyClosed,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            PrTab::All               => PrTab::All,
            PrTab::ByPeople          => PrTab::All,
            PrTab::ByRepo            => PrTab::ByPeople,
            PrTab::ReviewsRequested  => PrTab::ByRepo,
            PrTab::Watching          => PrTab::ReviewsRequested,
            PrTab::RecentlyClosed    => PrTab::Watching,
        }
    }
}

pub struct App {
    pub mode: Mode,
    pub should_quit: bool,
    pub needs_clear: bool,
    pub orgs: Vec<String>,
    pub search: SearchState,
    pub collected_repos: Vec<String>,
    pub focus: Focus,
    pub picker_insert: bool,
    pub preview_commit_selected: usize,
    pub diff_lines: Vec<String>,
    pub diff_loading: bool,
    pub diff_scroll: usize,
    pub diff_header: String,
    pub diff_repo: String,
    pub diff_sha: String,
    pub diff_url: Option<String>,
    pub diff_pr_number: Option<u32>,
    pub diff_jira: Option<String>,
    pub approved_prs: HashSet<String>,
    pub summary: String,
    pub summary_loading: bool,
    pub diff_prompt_active: bool,
    pub diff_prompt_input: String,
    pub diff_answer: String,
    pub diff_answer_loading: bool,
    pub repo_prompt_active: bool,
    pub repo_prompt_input: String,
    pub repo_answer: String,
    pub repo_answer_loading: bool,
    pub repo_progress: Vec<String>,
    pub repo_answer_scroll: usize,
    pub repo_conversation: Vec<(String, String)>,
    pub repo_current_question: String,
    pub pr_tab: PrTab,
    pub pr_items: Vec<PrItem>,
    pub pr_loading: bool,
    pub pr_error: Option<String>,
    pub pr_all_selected: usize,
    pub pr_reviews_items: Vec<PrItem>,
    pub pr_reviews_loading: bool,
    pub pr_reviews_error: Option<String>,
    pub pr_reviews_selected: usize,
    pub pr_people_query: String,
    pub pr_people_selected: usize,
    pub pr_repo_query: String,
    pub pr_repo_selected: usize,
    pub pr_watching_selected: usize,
    pub watched_repos: HashSet<String>,
    pub pr_closed_items: Vec<PrItem>,
    pub pr_closed_loading: bool,
    pub pr_closed_error: Option<String>,
    pub pr_closed_selected: usize,
    pub news_content: String,
    pub news_loading: bool,
    pub news_error: Option<String>,
    pub news_scroll: usize,
    pub model_list: Vec<String>,
    pub model_selected: usize,
    pub active_model: Option<String>,
    github: Option<Arc<GithubClient>>,
    llm: Option<Arc<LlmClient>>,
    debounce: Option<Instant>,
    preview_debounce: Option<Instant>,
    preview_cache: HashMap<String, RepoPreview>,
    diff_tx: mpsc::Sender<Result<String, String>>,
    diff_rx: mpsc::Receiver<Result<String, String>>,
    summary_tx: mpsc::Sender<Result<String, String>>,
    summary_rx: mpsc::Receiver<Result<String, String>>,
    answer_tx: mpsc::Sender<Result<String, String>>,
    answer_rx: mpsc::Receiver<Result<String, String>>,
    repo_answer_tx: mpsc::Sender<Result<String, String>>,
    repo_answer_rx: mpsc::Receiver<Result<String, String>>,
    repo_progress_tx: mpsc::Sender<String>,
    repo_progress_rx: mpsc::Receiver<String>,
    pr_tx: mpsc::Sender<Result<Vec<PrItem>, String>>,
    pr_rx: mpsc::Receiver<Result<Vec<PrItem>, String>>,
    pr_reviews_tx: mpsc::Sender<Result<Vec<PrItem>, String>>,
    pr_reviews_rx: mpsc::Receiver<Result<Vec<PrItem>, String>>,
    pr_closed_tx: mpsc::Sender<Result<Vec<PrItem>, String>>,
    pr_closed_rx: mpsc::Receiver<Result<Vec<PrItem>, String>>,
    watch_rx: mpsc::Receiver<Result<HashSet<String>, String>>,
    news_tx: mpsc::Sender<Result<String, String>>,
    news_rx: mpsc::Receiver<Result<String, String>>,
    current_user: Option<String>,
    user_rx: mpsc::Receiver<Result<String, String>>,
    pub diff_approval_loading: bool,
    approval_tx: mpsc::Sender<Result<(String, u32, bool), String>>,
    approval_rx: mpsc::Receiver<Result<(String, u32, bool), String>>,
    pub diff_comment_active: bool,
    pub diff_comment_input: String,
    pub diff_comment_submitted: bool,
    comment_tx: mpsc::Sender<Result<(), String>>,
    comment_rx: mpsc::Receiver<Result<(), String>>,
    pub diff_pr_comments: Vec<PrComment>,
    pub diff_pr_comments_loading: bool,
    pr_comments_tx: mpsc::Sender<Result<Vec<PrComment>, String>>,
    pr_comments_rx: mpsc::Receiver<Result<Vec<PrComment>, String>>,
    pub cache_size_bytes: Option<u64>,
}

impl App {
    pub fn new() -> Self {
        let (orgs, github, model_list) = match Config::load() {
            Ok(c) => {
                let orgs = c.github.orgs.clone();
                let gh = GithubClient::new(c.github.token, c.github.orgs).ok().map(Arc::new);
                (orgs, gh, c.model.models)
            }
            Err(_) => (Vec::new(), None, Vec::new()),
        };
        let active_model = load_saved_model();
        let llm = LlmClient::from_claude_settings(active_model.clone()).ok().map(Arc::new);
        let approved_prs = load_approved_prs();
        let (diff_tx, diff_rx) = mpsc::channel(4);
        let (summary_tx, summary_rx) = mpsc::channel(4);
        let (answer_tx, answer_rx) = mpsc::channel(4);
        let (repo_answer_tx, repo_answer_rx) = mpsc::channel(4);
        let (repo_progress_tx, repo_progress_rx) = mpsc::channel(64);
        let (pr_tx, pr_rx) = mpsc::channel(4);
        let (pr_reviews_tx, pr_reviews_rx) = mpsc::channel(4);
        let (pr_closed_tx, pr_closed_rx) = mpsc::channel(4);
        let (watch_tx, watch_rx) = mpsc::channel(4);
        let (news_tx, news_rx) = mpsc::channel(4);
        let (user_tx, user_rx) = mpsc::channel(2);
        let (approval_tx, approval_rx) = mpsc::channel(4);
        let (comment_tx, comment_rx) = mpsc::channel(4);
        let (pr_comments_tx, pr_comments_rx) = mpsc::channel(4);
        if let Some(ref gh) = github {
            let client = Arc::clone(gh);
            let tx = watch_tx.clone();
            tokio::spawn(async move {
                let result = client.fetch_watched_repos().await.map_err(|e| e.to_string());
                let _ = tx.send(result).await;
            });
            let client = Arc::clone(gh);
            tokio::spawn(async move {
                let result = client.get_current_user().await.map_err(|e| e.to_string());
                let _ = user_tx.send(result).await;
            });
        }
        Self {
            mode: Mode::Normal,
            should_quit: false,
            needs_clear: false,
            orgs,
            search: SearchState::new(),
            collected_repos: Vec::new(),
            focus: Focus::Results,
            picker_insert: true,
            preview_commit_selected: 0,
            diff_lines: Vec::new(),
            diff_loading: false,
            diff_scroll: 0,
            diff_header: String::new(),
            diff_repo: String::new(),
            diff_sha: String::new(),
            diff_url: None,
            diff_pr_number: None,
            diff_jira: None,
            approved_prs,
            summary: String::new(),
            summary_loading: false,
            diff_prompt_active: false,
            diff_prompt_input: String::new(),
            diff_answer: String::new(),
            diff_answer_loading: false,
            repo_prompt_active: false,
            repo_prompt_input: String::new(),
            repo_answer: String::new(),
            repo_answer_loading: false,
            repo_progress: Vec::new(),
            repo_answer_scroll: 0,
            repo_conversation: Vec::new(),
            repo_current_question: String::new(),
            pr_tab: PrTab::All,
            pr_items: Vec::new(),
            pr_loading: false,
            pr_error: None,
            pr_all_selected: 0,
            pr_reviews_items: Vec::new(),
            pr_reviews_loading: false,
            pr_reviews_error: None,
            pr_reviews_selected: 0,
            pr_people_query: String::new(),
            pr_people_selected: 0,
            pr_repo_query: String::new(),
            pr_repo_selected: 0,
            pr_watching_selected: 0,
            watched_repos: HashSet::new(),
            pr_closed_items: Vec::new(),
            pr_closed_loading: false,
            pr_closed_error: None,
            pr_closed_selected: 0,
            news_content: String::new(),
            news_loading: false,
            news_error: None,
            news_scroll: 0,
            model_list,
            model_selected: 0,
            active_model,
            current_user: None,
            user_rx,
            diff_approval_loading: false,
            approval_tx,
            approval_rx,
            diff_comment_active: false,
            diff_comment_input: String::new(),
            diff_comment_submitted: false,
            comment_tx,
            comment_rx,
            diff_pr_comments: Vec::new(),
            diff_pr_comments_loading: false,
            pr_comments_tx,
            pr_comments_rx,
            github,
            llm,
            debounce: None,
            preview_debounce: None,
            preview_cache: HashMap::new(),
            diff_tx,
            diff_rx,
            summary_tx,
            summary_rx,
            answer_tx,
            answer_rx,
            repo_answer_tx,
            repo_answer_rx,
            repo_progress_tx,
            repo_progress_rx,
            pr_tx,
            pr_rx,
            pr_reviews_tx,
            pr_reviews_rx,
            pr_closed_tx,
            pr_closed_rx,
            watch_rx,
            news_tx,
            news_rx,
            cache_size_bytes: norsedata_size(),
        }
    }

    pub fn llm_model(&self) -> Option<&str> {
        self.llm.as_ref().map(|c| c.active_model())
    }

    pub fn current_diff_approved(&self) -> bool {
        if let Some(number) = self.diff_pr_number {
            self.approved_prs.contains(&format!("{}#{}", self.diff_repo, number))
        } else {
            false
        }
    }

    pub fn poll(&mut self) {
        self.search.poll();

        if self.search.results_changed {
            self.search.results_changed = false;
            self.focus = Focus::Results;
            self.preview_commit_selected = 0;
            self.on_selection_change();
        }

        while let Ok((repo, result)) = self.search.preview_rx.try_recv() {
            let selected_repo = self.search.results.get(self.search.selected).map(|r| r.repo.as_str());
            match result {
                Ok(preview) => {
                    self.preview_cache.insert(repo.clone(), preview.clone());
                    if selected_repo == Some(repo.as_str()) {
                        self.search.preview = Some(preview);
                        self.search.preview_loading = false;
                    }
                }
                Err(_) => {
                    if selected_repo == Some(repo.as_str()) {
                        self.search.preview_loading = false;
                    }
                }
            }
        }

        // When diff arrives, kick off LLM summary.
        while let Ok(result) = self.diff_rx.try_recv() {
            self.diff_loading = false;
            match result {
                Ok(diff) => {
                    if let Some(ref client) = self.llm {
                        let client = Arc::clone(client);
                        let tx = self.summary_tx.clone();
                        let diff_for_llm = diff.clone();
                        self.summary.clear();
                        self.summary_loading = true;
                        // If we have a cached summary for this exact PR, show it immediately
                        // and skip the network call; the cache is already fresh.
                        if let Some(number) = self.diff_pr_number {
                            if let Some(cached) = load_pr_cache(&self.diff_repo, number) {
                                self.summary = cached.summary.clone();
                                self.summary_loading = false;
                            } else {
                                let repo_ctx = load_repo_context(&self.diff_repo, number);
                                tokio::spawn(async move {
                                    let result = client.summarize_diff(&diff_for_llm, repo_ctx).await.map_err(|e| e.to_string());
                                    let _ = tx.send(result).await;
                                });
                            }
                        } else {
                            tokio::spawn(async move {
                                let result = client.summarize_diff(&diff_for_llm, None).await.map_err(|e| e.to_string());
                                let _ = tx.send(result).await;
                            });
                        }
                    }
                    self.diff_lines = diff.lines().map(String::from).collect();
                }
                Err(e) => self.diff_lines = vec![format!("error: {}", e)],
            }
        }

        while let Ok(result) = self.summary_rx.try_recv() {
            self.summary_loading = false;
            match result {
                Ok(text) => {
                    if let Some(number) = self.diff_pr_number {
                        save_pr_cache(&self.diff_repo, number, &text);
                    }
                    self.summary = text;
                }
                Err(e) => self.summary = format!("error: {}", e),
            }
        }

        while let Ok(result) = self.answer_rx.try_recv() {
            self.diff_answer_loading = false;
            match result {
                Ok(text) => self.diff_answer = text,
                Err(e) => self.diff_answer = format!("error: {}", e),
            }
        }

        while let Ok(msg) = self.repo_progress_rx.try_recv() {
            self.repo_progress.push(msg);
        }

        while let Ok(result) = self.pr_rx.try_recv() {
            self.pr_loading = false;
            match result {
                Ok(prs) => {
                    self.pr_items = prs;
                    self.pr_all_selected = 0;
                    self.pr_people_selected = 0;
                    self.pr_repo_selected = 0;
                }
                Err(e) => self.pr_error = Some(e),
            }
        }

        while let Ok(result) = self.pr_reviews_rx.try_recv() {
            self.pr_reviews_loading = false;
            match result {
                Ok(prs) => { self.pr_reviews_items = prs; self.pr_reviews_selected = 0; }
                Err(e) => self.pr_reviews_error = Some(e),
            }
        }

        while let Ok(result) = self.pr_closed_rx.try_recv() {
            self.pr_closed_loading = false;
            match result {
                Ok(prs) => { self.pr_closed_items = prs; self.pr_closed_selected = 0; }
                Err(e) => self.pr_closed_error = Some(e),
            }
        }

        while let Ok(result) = self.watch_rx.try_recv() {
            if let Ok(repos) = result {
                self.watched_repos = repos;
            }
        }

        while let Ok(result) = self.news_rx.try_recv() {
            self.news_loading = false;
            match result {
                Ok(content) => self.news_content = content,
                Err(e) => self.news_error = Some(e),
            }
        }

        while let Ok(result) = self.user_rx.try_recv() {
            if let Ok(login) = result {
                self.current_user = Some(login);
            }
        }

        while let Ok(result) = self.approval_rx.try_recv() {
            self.diff_approval_loading = false;
            if let Ok((repo, number, approved)) = result {
                let key = format!("{}#{}", repo, number);
                if approved && !self.approved_prs.contains(&key) {
                    persist_approved_pr(&key);
                    self.approved_prs.insert(key);
                }
            }
        }

        while let Ok(_result) = self.comment_rx.try_recv() {
            self.diff_comment_submitted = false;
            self.diff_comment_active = false;
            self.diff_comment_input.clear();
        }

        while let Ok(result) = self.pr_comments_rx.try_recv() {
            self.diff_pr_comments_loading = false;
            if let Ok(comments) = result {
                self.diff_pr_comments = comments;
            }
        }

        while let Ok(result) = self.repo_answer_rx.try_recv() {
            self.repo_answer_loading = false;
            let text = match result {
                Ok(text) => text,
                Err(e) => format!("error: {}", e),
            };
            self.repo_conversation.push((self.repo_current_question.clone(), text.clone()));
            self.repo_answer = text;
        }

        if let Some(t) = self.debounce {
            if t.elapsed() >= Duration::from_millis(300) {
                self.debounce = None;
                self.fire_search();
            }
        }

        if let Some(t) = self.preview_debounce {
            if t.elapsed() >= Duration::from_millis(500) {
                self.preview_debounce = None;
                self.fire_preview();
            }
        }
    }

    // Returns the combined display list: uncollected search results first, then collected repos.
    pub fn picker_display(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.search.results.iter()
            .map(|r| r.repo.as_str())
            .filter(|r| !self.collected_repos.iter().any(|c| c.as_str() == *r))
            .collect();
        for repo in &self.collected_repos {
            v.push(repo.as_str());
        }
        v
    }

    fn selected_picker_repo(&self) -> Option<String> {
        let idx = self.search.selected;
        let uncollected_count = self.search.results.iter()
            .filter(|r| !self.collected_repos.iter().any(|c| c == &r.repo))
            .count();
        if idx < uncollected_count {
            self.search.results.iter()
                .filter(|r| !self.collected_repos.iter().any(|c| c == &r.repo))
                .nth(idx)
                .map(|r| r.repo.clone())
        } else {
            self.collected_repos.get(idx - uncollected_count).cloned()
        }
    }

    fn toggle_collect(&mut self) {
        let Some(repo) = self.selected_picker_repo() else { return; };
        if let Some(pos) = self.collected_repos.iter().position(|r| r == &repo) {
            self.collected_repos.remove(pos);
            let len = self.picker_display().len();
            if len > 0 && self.search.selected >= len {
                self.search.selected = len - 1;
            }
        } else {
            self.collected_repos.push(repo);
        }
    }

    fn on_selection_change(&mut self) {
        self.preview_commit_selected = 0;
        let Some(repo) = self.selected_picker_repo() else {
            self.search.preview = None;
            self.search.preview_loading = false;
            return;
        };

        if let Some(cached) = self.preview_cache.get(&repo) {
            self.search.preview = Some(cached.clone());
            self.search.preview_loading = false;
        } else {
            self.search.preview = None;
            self.preview_debounce = Some(Instant::now());
        }
    }

    fn fire_search(&mut self) {
        if self.search.query.is_empty() {
            self.search.results.clear();
            self.search.loading = false;
            return;
        }

        let Some(client) = self.github.clone() else {
            self.search.error = Some("GITHUB_TOKEN not set".into());
            return;
        };

        let query = self.search.query.clone();
        let tx = self.search.tx.clone();
        self.search.loading = true;
        self.search.error = None;

        tokio::spawn(async move {
            let result = client
                .search_repos(&query)
                .await
                .map(|repos| repos.into_iter().map(|repo| SearchResult { repo }).collect())
                .map_err(|e| e.to_string());
            let _ = tx.send(result).await;
        });
    }

    fn fire_preview(&mut self) {
        let Some(repo) = self.selected_picker_repo() else { return; };
        let Some(client) = self.github.clone() else { return; };
        let tx = self.search.preview_tx.clone();
        self.search.preview_loading = true;

        tokio::spawn(async move {
            let result = client.get_repo_preview(&repo).await.map_err(|e| e.to_string());
            let _ = tx.send((repo, result)).await;
        });
    }

    pub fn pr_people_filtered(&self) -> Vec<&PrItem> {
        let q = self.pr_people_query.to_lowercase();
        let mut v: Vec<&PrItem> = self.pr_items.iter()
            .filter(|p| q.is_empty() || p.author.to_lowercase().contains(&q))
            .collect();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        v
    }

    pub fn pr_repo_filtered(&self) -> Vec<&PrItem> {
        let q = self.pr_repo_query.to_lowercase();
        let mut v: Vec<&PrItem> = self.pr_items.iter()
            .filter(|p| q.is_empty() || p.repo.to_lowercase().contains(&q))
            .collect();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        v
    }

    pub fn pr_watching_filtered(&self) -> Vec<&PrItem> {
        let mut v: Vec<&PrItem> = self.pr_items.iter()
            .filter(|p| self.watched_repos.contains(&p.repo))
            .collect();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        v
    }

    fn toggle_watch(&mut self, repo: String) {
        let Some(client) = self.github.clone() else { return; };
        let Some((owner, name)) = repo.split_once('/').map(|(o, n)| (o.to_string(), n.to_string())) else { return; };
        if self.watched_repos.contains(&repo) {
            self.watched_repos.remove(&repo);
            tokio::spawn(async move { let _ = client.unwatch_repo(&owner, &name).await; });
        } else {
            self.watched_repos.insert(repo);
            tokio::spawn(async move { let _ = client.watch_repo(&owner, &name).await; });
        }
    }

    fn handle_pr_browser(&mut self, key: KeyEvent) {
        // Tab switching always takes priority
        if key.modifiers == KeyModifiers::CONTROL {
            match key.code {
                KeyCode::Char('l') => {
                    self.pr_tab = self.pr_tab.next();
                    self.on_pr_tab_enter();
                    return;
                }
                KeyCode::Char('h') => {
                    self.pr_tab = self.pr_tab.prev();
                    self.on_pr_tab_enter();
                    return;
                }
                _ => {}
            }
        }
        match self.pr_tab {
            PrTab::All              => self.handle_pr_all(key),
            PrTab::ByPeople         => self.handle_pr_people(key),
            PrTab::ByRepo           => self.handle_pr_repo(key),
            PrTab::ReviewsRequested => self.handle_pr_reviews(key),
            PrTab::Watching         => self.handle_pr_watching(key),
            PrTab::RecentlyClosed   => self.handle_pr_closed(key),
        }
    }

    fn on_pr_tab_enter(&mut self) {
        if matches!(self.pr_tab, PrTab::RecentlyClosed)
            && !self.pr_closed_loading
            && self.pr_closed_items.is_empty()
            && self.pr_closed_error.is_none()
        {
            self.fire_closed_load();
        }
    }

    fn handle_pr_closed(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::Char('q')) => {
                self.mode = Mode::Normal;
                self.needs_clear = true;
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                let len = self.pr_closed_items.len();
                if len > 0 { self.pr_closed_selected = (self.pr_closed_selected + 1).min(len - 1); }
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                self.pr_closed_selected = self.pr_closed_selected.saturating_sub(1);
            }
            (_, KeyCode::Enter) => {
                if let Some(pr) = self.pr_closed_items.get(self.pr_closed_selected).cloned() {
                    self.open_pr_diff(&pr);
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                if let Some(pr) = self.pr_closed_items.get(self.pr_closed_selected) {
                    open_in_browser(&pr.html_url.clone());
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => self.fire_closed_load(),
            _ => {}
        }
    }

    fn fire_closed_load(&mut self) {
        let Some(client) = self.github.clone() else {
            self.pr_closed_error = Some("no github client configured".into());
            return;
        };
        let tx = self.pr_closed_tx.clone();
        self.pr_closed_loading = true;
        self.pr_closed_error = None;
        tokio::spawn(async move {
            let result = client.fetch_recently_closed_prs().await.map_err(|e| e.to_string());
            let _ = tx.send(result).await;
        });
    }

    fn handle_pr_reviews(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::Char('q')) => {
                self.mode = Mode::Normal;
                self.needs_clear = true;
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                let len = self.pr_reviews_items.len();
                if len > 0 { self.pr_reviews_selected = (self.pr_reviews_selected + 1).min(len - 1); }
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                self.pr_reviews_selected = self.pr_reviews_selected.saturating_sub(1);
            }
            (_, KeyCode::Enter) => {
                if let Some(pr) = self.pr_reviews_items.get(self.pr_reviews_selected).cloned() {
                    self.open_pr_diff(&pr);
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                if let Some(pr) = self.pr_reviews_items.get(self.pr_reviews_selected) {
                    open_in_browser(&pr.html_url.clone());
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => self.fire_pr_load(),
            _ => {}
        }
    }

    fn handle_pr_all(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::Char('q')) => {
                self.mode = Mode::Normal;
                self.needs_clear = true;
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                let len = self.pr_items.len();
                if len > 0 { self.pr_all_selected = (self.pr_all_selected + 1).min(len - 1); }
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                self.pr_all_selected = self.pr_all_selected.saturating_sub(1);
            }
            (_, KeyCode::Enter) => {
                if let Some(pr) = self.pr_items.get(self.pr_all_selected).cloned() {
                    self.open_pr_diff(&pr);
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                if let Some(pr) = self.pr_items.get(self.pr_all_selected) {
                    open_in_browser(&pr.html_url.clone());
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => self.fire_pr_load(),
            _ => {}
        }
    }

    fn handle_pr_people(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                if self.pr_people_query.is_empty() {
                    self.mode = Mode::Normal;
                    self.needs_clear = true;
                } else {
                    self.pr_people_query.clear();
                    self.pr_people_selected = 0;
                }
            }
            (_, KeyCode::Up) => {
                self.pr_people_selected = self.pr_people_selected.saturating_sub(1);
            }
            (_, KeyCode::Down) => {
                let len = self.pr_people_filtered().len();
                if len > 0 { self.pr_people_selected = (self.pr_people_selected + 1).min(len - 1); }
            }
            (_, KeyCode::Enter) => {
                if let Some(pr) = self.pr_people_filtered().get(self.pr_people_selected).cloned().cloned() {
                    self.open_pr_diff(&pr);
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                let filtered = self.pr_people_filtered();
                if let Some(pr) = filtered.get(self.pr_people_selected) {
                    open_in_browser(&pr.html_url.clone());
                }
            }
            (_, KeyCode::Backspace) => {
                self.pr_people_query.pop();
                self.pr_people_selected = 0;
            }
            (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                self.pr_people_query.push(c);
                self.pr_people_selected = 0;
            }
            _ => {}
        }
    }

    fn handle_pr_repo(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                if self.pr_repo_query.is_empty() {
                    self.mode = Mode::Normal;
                    self.needs_clear = true;
                } else {
                    self.pr_repo_query.clear();
                    self.pr_repo_selected = 0;
                }
            }
            (_, KeyCode::Up) => {
                self.pr_repo_selected = self.pr_repo_selected.saturating_sub(1);
            }
            (_, KeyCode::Down) => {
                let len = self.pr_repo_filtered().len();
                if len > 0 { self.pr_repo_selected = (self.pr_repo_selected + 1).min(len - 1); }
            }
            (_, KeyCode::Enter) => {
                if let Some(pr) = self.pr_repo_filtered().get(self.pr_repo_selected).cloned().cloned() {
                    self.open_pr_diff(&pr);
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                let filtered = self.pr_repo_filtered();
                if let Some(pr) = filtered.get(self.pr_repo_selected) {
                    open_in_browser(&pr.html_url.clone());
                }
            }
            (_, KeyCode::Backspace) => {
                self.pr_repo_query.pop();
                self.pr_repo_selected = 0;
            }
            (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                self.pr_repo_query.push(c);
                self.pr_repo_selected = 0;
            }
            _ => {}
        }
    }

    fn handle_pr_watching(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::Char('q')) => {
                self.mode = Mode::Normal;
                self.needs_clear = true;
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                let len = self.pr_watching_filtered().len();
                if len > 0 { self.pr_watching_selected = (self.pr_watching_selected + 1).min(len - 1); }
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                self.pr_watching_selected = self.pr_watching_selected.saturating_sub(1);
            }
            (_, KeyCode::Enter) => {
                if let Some(pr) = self.pr_watching_filtered().get(self.pr_watching_selected).cloned().cloned() {
                    self.open_pr_diff(&pr);
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                let prs = self.pr_watching_filtered();
                if let Some(pr) = prs.get(self.pr_watching_selected) {
                    open_in_browser(&pr.html_url.clone());
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => self.fire_pr_load(),
            _ => {}
        }
    }

    fn fire_pr_load(&mut self) {
        let Some(client) = self.github.clone() else {
            self.pr_error = Some("no github client configured".into());
            self.pr_reviews_error = Some("no github client configured".into());
            return;
        };
        let pr_tx = self.pr_tx.clone();
        let reviews_tx = self.pr_reviews_tx.clone();
        self.pr_loading = true;
        self.pr_reviews_loading = true;
        self.pr_error = None;
        self.pr_reviews_error = None;
        let c1 = Arc::clone(&client);
        let c2 = Arc::clone(&client);
        tokio::spawn(async move {
            let result = c1.search_prs().await.map_err(|e| e.to_string());
            let _ = pr_tx.send(result).await;
        });
        tokio::spawn(async move {
            let result = c2.search_reviews_requested().await.map_err(|e| e.to_string());
            let _ = reviews_tx.send(result).await;
        });
    }

    fn open_pr_diff(&mut self, pr: &PrItem) {
        let repo = pr.repo.clone();
        let number = pr.number;
        self.diff_header = format!("{}  #{} {}", repo, number, pr.title);
        self.diff_repo = repo.clone();
        self.diff_sha = String::new();
        self.diff_url = Some(pr.html_url.clone());
        self.diff_pr_number = Some(number);
        self.diff_jira = extract_jira(&pr.title);
        self.diff_lines.clear();
        self.diff_loading = true;
        self.diff_scroll = 0;
        self.summary.clear();
        self.summary_loading = self.llm.is_some();
        self.diff_prompt_active = false;
        self.diff_prompt_input.clear();
        self.diff_answer.clear();
        self.diff_answer_loading = false;
        self.mode = Mode::Diff;

        // Fetch PR comments.
        self.diff_pr_comments.clear();
        if let Some(client) = self.github.clone() {
            let tx = self.pr_comments_tx.clone();
            let fetch_repo = repo.clone();
            self.diff_pr_comments_loading = true;
            tokio::spawn(async move {
                let result = client.get_pr_comments(&fetch_repo, number).await.map_err(|e| e.to_string());
                let _ = tx.send(result).await;
            });
        }

        // Check GitHub for the live approval state (catches approvals made outside norse).
        if let (Some(client), Some(ref username)) = (self.github.clone(), self.current_user.clone()) {
            let tx = self.approval_tx.clone();
            let check_repo = repo.clone();
            let username = username.clone();
            self.diff_approval_loading = true;
            tokio::spawn(async move {
                let result = client.is_pr_approved_by(&check_repo, number, &username).await
                    .map(|approved| (check_repo, number, approved))
                    .map_err(|e| e.to_string());
                let _ = tx.send(result).await;
            });
        }

        let Some(client) = self.github.clone() else {
            self.diff_loading = false;
            self.diff_lines = vec!["no github client configured".into()];
            return;
        };

        let tx = self.diff_tx.clone();
        tokio::spawn(async move {
            let result = client.get_pr_diff(&repo, number).await.map_err(|e| e.to_string());
            let _ = tx.send(result).await;
        });
    }

    fn open_diff(&mut self) {
        let sha = match self.search.preview.as_ref()
            .and_then(|p| p.commits.get(self.preview_commit_selected))
        {
            Some(c) => c.sha.clone(),
            None => return,
        };
        let Some(repo) = self.selected_picker_repo() else { return; };

        self.diff_header = format!("{}  {}", repo, sha);
        self.diff_repo = repo.clone();
        self.diff_sha = sha.clone();
        self.diff_url = None;
        self.diff_pr_number = None;
        self.diff_jira = None;
        self.diff_approval_loading = false;
        self.diff_pr_comments.clear();
        self.diff_pr_comments_loading = false;
        self.diff_lines.clear();
        self.diff_loading = true;
        self.diff_scroll = 0;
        self.summary.clear();
        self.summary_loading = self.llm.is_some();
        self.diff_prompt_active = false;
        self.diff_prompt_input.clear();
        self.diff_answer.clear();
        self.diff_answer_loading = false;
        self.mode = Mode::Diff;

        let Some(client) = self.github.clone() else {
            self.diff_loading = false;
            self.diff_lines = vec!["no github client configured".into()];
            return;
        };

        let tx = self.diff_tx.clone();
        tokio::spawn(async move {
            let result = client.get_commit_diff(&repo, &sha).await.map_err(|e| e.to_string());
            let _ = tx.send(result).await;
        });
    }

    pub async fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Normal => self.handle_normal(key),
            Mode::Picker => self.handle_picker(key),
            Mode::Diff => self.handle_diff(key),
            Mode::PrBrowser => self.handle_pr_browser(key),
            Mode::News => self.handle_news(key),
            Mode::ModelPicker => self.handle_model_picker(key),
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.mode = Mode::Picker;
                self.search.reset();
                self.focus = Focus::Results;
                self.picker_insert = true;
                self.preview_commit_selected = 0;
            }
            (KeyModifiers::NONE, KeyCode::Char('p')) => {
                self.mode = Mode::PrBrowser;
                self.pr_tab = PrTab::All;
                self.pr_items.clear();
                self.pr_reviews_items.clear();
                self.pr_people_query.clear();
                self.pr_repo_query.clear();
                self.pr_all_selected = 0;
                self.pr_reviews_selected = 0;
                self.pr_people_selected = 0;
                self.pr_repo_selected = 0;
                self.fire_pr_load();
            }
            (KeyModifiers::NONE, KeyCode::Char('n')) => {
                self.mode = Mode::News;
                self.news_scroll = 0;
                self.news_content.clear();
                self.news_error = None;
                self.fire_news_load();
            }
            (KeyModifiers::NONE, KeyCode::Char('m')) => {
                if !self.model_list.is_empty() {
                    self.model_selected = self.model_list.iter().position(|m| {
                        Some(m) == self.active_model.as_ref()
                    }).unwrap_or(0);
                    self.mode = Mode::ModelPicker;
                }
            }
            (_, KeyCode::Char('q')) => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_model_picker(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::Char('q')) => {
                self.mode = Mode::Normal;
                self.needs_clear = true;
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                let len = self.model_list.len();
                if len > 0 { self.model_selected = (self.model_selected + 1).min(len - 1); }
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                self.model_selected = self.model_selected.saturating_sub(1);
            }
            (_, KeyCode::Enter) => {
                if let Some(model) = self.model_list.get(self.model_selected).cloned() {
                    save_model(&model);
                    self.active_model = Some(model.clone());
                    self.llm = LlmClient::from_claude_settings(Some(model)).ok().map(Arc::new);
                    self.mode = Mode::Normal;
                    self.needs_clear = true;
                }
            }
            _ => {}
        }
    }

    fn handle_picker(&mut self, key: KeyEvent) {
        if self.repo_prompt_active {
            match (key.modifiers, key.code) {
                (_, KeyCode::Esc) => {
                    self.repo_prompt_active = false;
                    self.repo_prompt_input.clear();
                }
                (_, KeyCode::Enter) => self.fire_repo_question(),
                (_, KeyCode::Backspace) => { self.repo_prompt_input.pop(); }
                (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                    if self.repo_answer_loading || !self.repo_conversation.is_empty() {
                        self.focus = Focus::Answer;
                    }
                }
                (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
                    self.focus = Focus::Results;
                }
                (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                    self.repo_prompt_input.push(c);
                }
                _ => {}
            }
            return;
        }

        if key.code == KeyCode::Esc {
            match (&self.focus, self.picker_insert) {
                (Focus::Answer, _) => self.focus = Focus::Results,
                (Focus::Preview, _) => self.focus = Focus::Results,
                (Focus::Results, true) => self.picker_insert = false,
                (Focus::Results, false) => self.mode = Mode::Normal,
            }
            return;
        }

        match &self.focus {
            Focus::Results if self.picker_insert => match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                    if self.repo_answer_loading || !self.repo_conversation.is_empty() {
                        self.focus = Focus::Answer;
                    } else if self.search.preview.as_ref().map(|p| !p.commits.is_empty()).unwrap_or(false) {
                        self.focus = Focus::Preview;
                        self.preview_commit_selected = 0;
                    }
                }
                (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                    self.search.query.push(c);
                    self.debounce = Some(Instant::now());
                }
                (_, KeyCode::Backspace) => {
                    self.search.query.pop();
                    self.debounce = Some(Instant::now());
                }
                _ => {}
            },
            Focus::Results => match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Char('i')) => self.picker_insert = true,
                (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                    let len = self.picker_display().len();
                    if len > 0 { self.search.selected = (self.search.selected + 1).min(len - 1); }
                    self.on_selection_change();
                }
                (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                    self.search.selected = self.search.selected.saturating_sub(1);
                    self.on_selection_change();
                }
                (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                    if self.repo_answer_loading || !self.repo_conversation.is_empty() {
                        self.focus = Focus::Answer;
                    } else if self.search.preview.as_ref().map(|p| !p.commits.is_empty()).unwrap_or(false) {
                        self.focus = Focus::Preview;
                        self.preview_commit_selected = 0;
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('/')) => {
                    if !self.collected_repos.is_empty() || self.selected_picker_repo().is_some() {
                        self.repo_prompt_active = true;
                        self.repo_prompt_input.clear();
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('c')) => self.toggle_collect(),
                (KeyModifiers::NONE, KeyCode::Char('w')) => {
                    if let Some(repo) = self.selected_picker_repo() {
                        self.toggle_watch(repo);
                    }
                }
                _ => {}
            },
            Focus::Preview => match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                    if let Some(ref preview) = self.search.preview {
                        self.preview_commit_selected = (self.preview_commit_selected + 1)
                            .min(preview.commits.len().saturating_sub(1));
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                    self.preview_commit_selected = self.preview_commit_selected.saturating_sub(1);
                }
                (_, KeyCode::Enter) => self.open_diff(),
                (KeyModifiers::CONTROL, KeyCode::Char('h')) => self.focus = Focus::Results,
                _ => {}
            },
            Focus::Answer => match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('h')) => self.focus = Focus::Results,
                (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                    self.repo_answer_scroll = self.repo_answer_scroll.saturating_add(1);
                }
                (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                    self.repo_answer_scroll = self.repo_answer_scroll.saturating_sub(1);
                }
                (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                    self.repo_answer_scroll = self.repo_answer_scroll.saturating_add(15);
                }
                (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                    self.repo_answer_scroll = self.repo_answer_scroll.saturating_sub(15);
                }
                _ => {}
            },
        }
    }

    fn handle_diff(&mut self, key: KeyEvent) {
        if self.diff_comment_active {
            match key.code {
                KeyCode::Esc => {
                    self.diff_comment_active = false;
                    self.diff_comment_input.clear();
                }
                KeyCode::Enter => self.fire_pr_comment(),
                KeyCode::Backspace => { self.diff_comment_input.pop(); }
                KeyCode::Char(c) => self.diff_comment_input.push(c),
                _ => {}
            }
            return;
        }

        if self.diff_prompt_active {
            match key.code {
                KeyCode::Esc => {
                    self.diff_prompt_active = false;
                    self.diff_prompt_input.clear();
                }
                KeyCode::Enter => self.fire_diff_question(),
                KeyCode::Backspace => { self.diff_prompt_input.pop(); }
                KeyCode::Char(c) => self.diff_prompt_input.push(c),
                _ => {}
            }
            return;
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::Char('q')) => {
                self.mode = if self.diff_url.is_some() { Mode::PrBrowser } else { Mode::Picker };
                self.needs_clear = true;
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                self.diff_scroll = self.diff_scroll.saturating_add(1);
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                self.diff_scroll = self.diff_scroll.saturating_sub(1);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                self.diff_scroll = self.diff_scroll.saturating_add(15);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                self.diff_scroll = self.diff_scroll.saturating_sub(15);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                if let Some(ref url) = self.diff_url {
                    open_in_browser(url);
                } else {
                    let repo = self.diff_repo.clone();
                    let sha = self.diff_sha.clone();
                    if let Some(ref client) = self.github {
                        let client = Arc::clone(client);
                        tokio::spawn(async move {
                            let url = client.get_pr_url(&repo, &sha).await
                                .unwrap_or_else(|_| format!("https://github.com/{}/commit/{}", repo, sha));
                            open_in_browser(&url);
                        });
                    }
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                if let Some(number) = self.diff_pr_number {
                    let repo = self.diff_repo.clone();
                    let key = format!("{}#{}", repo, number);
                    if !self.approved_prs.contains(&key) {
                        persist_approved_pr(&key);
                        self.approved_prs.insert(key);
                        if let Some(client) = self.github.clone() {
                            tokio::spawn(async move { let _ = client.approve_pr(&repo, number).await; });
                        }
                        self.mode = Mode::PrBrowser;
                        self.needs_clear = true;
                    }
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('/')) => {
                self.diff_prompt_active = true;
                self.diff_prompt_input.clear();
            }
            (_, KeyCode::Char('R')) if self.diff_pr_number.is_some() => {
                self.diff_comment_active = true;
                self.diff_comment_input.clear();
            }
            _ => {}
        }
    }

    fn fire_pr_comment(&mut self) {
        let body = self.diff_comment_input.trim().to_string();
        if body.is_empty() { return; }
        let Some(number) = self.diff_pr_number else { return; };
        let Some(client) = self.github.clone() else { return; };
        let repo = self.diff_repo.clone();
        let tx = self.comment_tx.clone();
        self.diff_comment_submitted = true;
        tokio::spawn(async move {
            let result = client.comment_on_pr(&repo, number, &body).await.map_err(|e| e.to_string());
            let _ = tx.send(result).await;
        });
    }

    fn fire_repo_question(&mut self) {
        let question = self.repo_prompt_input.trim().to_string();
        if question.is_empty() { return; }
        let Some(ref client) = self.llm else { return; };
        let client = Arc::clone(client);
        let tx = self.repo_answer_tx.clone();
        self.repo_current_question = question.clone();
        self.repo_prompt_input.clear();
        self.repo_answer_loading = true;
        self.repo_answer_scroll = 0;
        self.repo_progress.clear();
        let progress_tx = self.repo_progress_tx.clone();
        let history = self.repo_conversation.clone();

        if !self.collected_repos.is_empty() {
            let repos = self.collected_repos.clone();
            tokio::spawn(async move {
                let result = client.ask_repos(&repos, &question, &history, progress_tx).await.map_err(|e| e.to_string());
                let _ = tx.send(result).await;
            });
        } else {
            let Some(repo) = self.selected_picker_repo() else { return; };
            tokio::spawn(async move {
                let result = client.ask_repo(&repo, &question, &history, progress_tx).await.map_err(|e| e.to_string());
                let _ = tx.send(result).await;
            });
        }
    }

    fn handle_news(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::Char('q')) => {
                self.mode = Mode::Normal;
                self.needs_clear = true;
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                self.news_scroll = self.news_scroll.saturating_add(1);
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                self.news_scroll = self.news_scroll.saturating_sub(1);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                self.news_scroll = self.news_scroll.saturating_add(15);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                self.news_scroll = self.news_scroll.saturating_sub(15);
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.news_scroll = 0;
                self.news_content.clear();
                self.news_error = None;
                self.fire_news_load();
            }
            _ => {}
        }
    }

    fn fire_news_load(&mut self) {
        let Some(client) = self.github.clone() else {
            self.news_error = Some("no github client configured".into());
            return;
        };
        let tx = self.news_tx.clone();
        self.news_loading = true;
        tokio::spawn(async move {
            let activity = match client.fetch_news_activity().await {
                Ok(a) => a,
                Err(e) => { let _ = tx.send(Err(e.to_string())).await; return; }
            };
            let _ = tx.send(Ok(format_news_activity(&activity))).await;
        });
    }

    fn fire_diff_question(&mut self) {
        let question = self.diff_prompt_input.trim().to_string();
        if question.is_empty() { return; }
        let Some(ref client) = self.llm else { return; };
        let client = Arc::clone(client);
        let tx = self.answer_tx.clone();
        let diff = self.diff_lines.join("\n");
        let repo = self.diff_repo.clone();
        self.diff_prompt_active = false;
        self.diff_prompt_input.clear();
        self.diff_answer.clear();
        self.diff_answer_loading = true;
        tokio::spawn(async move {
            let result = client.ask_diff(&diff, &repo, &question).await.map_err(|e| e.to_string());
            let _ = tx.send(result).await;
        });
    }
}

fn format_news_activity(activity: &[RepoActivity]) -> String {
    if activity.is_empty() {
        return "No pull request activity found in the last 24 hours.".into();
    }
    let mut out = String::new();
    for ra in activity {
        if ra.prs.is_empty() { continue; }
        let watch = if ra.is_watched { " [watched]" } else { "" };
        out.push_str(&format!("REPO: {}{}\n", ra.repo, watch));
        for p in &ra.prs {
            let jira = p.jira.as_deref().map(|j| format!(" [{}]", j)).unwrap_or_default();
            out.push_str(&format!(
                "  PR #{} [{}]{} \"{}\" by {}\n  GitHub: {}\n",
                p.number, p.state, jira, p.title, p.author, p.html_url
            ));
            if let Some(ref body) = p.body {
                out.push_str(&format!("  Description: {}\n", body));
            }
        }
        out.push('\n');
    }
    if out.trim().is_empty() {
        "No pull request activity found in the last 24 hours.".into()
    } else {
        out
    }
}

fn load_saved_model() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let s = std::fs::read_to_string(format!("{}/.norsedata/model", home)).ok()?;
    let s = s.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn save_model(model: &str) {
    if let Ok(home) = std::env::var("HOME") {
        let dir = format!("{}/.norsedata", home);
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(format!("{}/model", dir), model);
    }
}

fn load_approved_prs() -> HashSet<String> {
    let home = match std::env::var("HOME") { Ok(h) => h, Err(_) => return HashSet::new() };
    let text = match std::fs::read_to_string(format!("{}/.norsedata/prs-approved", home)) {
        Ok(t) => t,
        Err(_) => return HashSet::new(),
    };
    text.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect()
}

fn persist_approved_pr(key: &str) {
    if let Ok(home) = std::env::var("HOME") {
        let dir = format!("{}/.norsedata", home);
        let _ = std::fs::create_dir_all(&dir);
        let path = format!("{}/prs-approved", dir);
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path);
        if let Ok(ref mut f) = file {
            use std::io::Write;
            let _ = writeln!(f, "{}", key);
        }
    }
}

fn open_in_browser(url: &str) {
    #[cfg(target_os = "macos")]
    { std::process::Command::new("open").args(["-na", "Google Chrome", "--args", "--new-window", url]).spawn().ok(); }
    #[cfg(target_os = "linux")]
    { std::process::Command::new("xdg-open").arg(url).spawn().ok(); }
    #[cfg(target_os = "windows")]
    { std::process::Command::new("cmd").args(["/c", "start", url]).spawn().ok(); }
}
