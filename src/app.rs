use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use crate::config::Config;
use crate::github::{GithubClient, PrItem, RepoPreview};
use crate::llm::LlmClient;
use crate::search::{SearchResult, SearchState};

pub enum Mode {
    Normal,
    Picker,
    Diff,
    PrBrowser,
}

pub enum Focus {
    Results,
    Preview,
}

pub enum PrTab {
    All,
    ByPeople,
    ByRepo,
    ReviewsRequested,
    Watching,
}

impl PrTab {
    pub fn next(&self) -> Self {
        match self {
            PrTab::All               => PrTab::ByPeople,
            PrTab::ByPeople          => PrTab::ByRepo,
            PrTab::ByRepo            => PrTab::ReviewsRequested,
            PrTab::ReviewsRequested  => PrTab::Watching,
            PrTab::Watching          => PrTab::Watching,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            PrTab::All               => PrTab::All,
            PrTab::ByPeople          => PrTab::All,
            PrTab::ByRepo            => PrTab::ByPeople,
            PrTab::ReviewsRequested  => PrTab::ByRepo,
            PrTab::Watching          => PrTab::ReviewsRequested,
        }
    }
}

pub struct App {
    pub mode: Mode,
    pub should_quit: bool,
    pub needs_clear: bool,
    pub orgs: Vec<String>,
    pub search: SearchState,
    pub focus: Focus,
    pub picker_insert: bool,
    pub preview_commit_selected: usize,
    pub diff_lines: Vec<String>,
    pub diff_loading: bool,
    pub diff_scroll: usize,
    pub diff_header: String,
    pub diff_repo: String,
    pub diff_sha: String,
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
    watch_rx: mpsc::Receiver<Result<HashSet<String>, String>>,
}

impl App {
    pub fn new() -> Self {
        let (orgs, github) = match Config::load() {
            Ok(c) => {
                let orgs = c.github.orgs.clone();
                let gh = GithubClient::new(c.github.token, c.github.orgs).ok().map(Arc::new);
                (orgs, gh)
            }
            Err(_) => (Vec::new(), None),
        };
        let llm = LlmClient::from_claude_settings().ok().map(Arc::new);
        let (diff_tx, diff_rx) = mpsc::channel(4);
        let (summary_tx, summary_rx) = mpsc::channel(4);
        let (answer_tx, answer_rx) = mpsc::channel(4);
        let (repo_answer_tx, repo_answer_rx) = mpsc::channel(4);
        let (repo_progress_tx, repo_progress_rx) = mpsc::channel(64);
        let (pr_tx, pr_rx) = mpsc::channel(4);
        let (pr_reviews_tx, pr_reviews_rx) = mpsc::channel(4);
        let (watch_tx, watch_rx) = mpsc::channel(4);
        if let Some(ref gh) = github {
            let client = Arc::clone(gh);
            let tx = watch_tx.clone();
            tokio::spawn(async move {
                let result = client.fetch_watched_repos().await.map_err(|e| e.to_string());
                let _ = tx.send(result).await;
            });
        }
        Self {
            mode: Mode::Normal,
            should_quit: false,
            needs_clear: false,
            orgs,
            search: SearchState::new(),
            focus: Focus::Results,
            picker_insert: true,
            preview_commit_selected: 0,
            diff_lines: Vec::new(),
            diff_loading: false,
            diff_scroll: 0,
            diff_header: String::new(),
            diff_repo: String::new(),
            diff_sha: String::new(),
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
            watch_rx,
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
                        tokio::spawn(async move {
                            let result = client.summarize_diff(&diff_for_llm).await.map_err(|e| e.to_string());
                            let _ = tx.send(result).await;
                        });
                    }
                    self.diff_lines = diff.lines().map(String::from).collect();
                }
                Err(e) => self.diff_lines = vec![format!("error: {}", e)],
            }
        }

        while let Ok(result) = self.summary_rx.try_recv() {
            self.summary_loading = false;
            match result {
                Ok(text) => self.summary = text,
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

        while let Ok(result) = self.watch_rx.try_recv() {
            if let Ok(repos) = result {
                self.watched_repos = repos;
            }
        }

        while let Ok(result) = self.repo_answer_rx.try_recv() {
            self.repo_answer_loading = false;
            match result {
                Ok(text) => self.repo_answer = text,
                Err(e) => self.repo_answer = format!("error: {}", e),
            }
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

    fn on_selection_change(&mut self) {
        self.preview_commit_selected = 0;
        let Some(result) = self.search.results.get(self.search.selected) else {
            self.search.preview = None;
            self.search.preview_loading = false;
            return;
        };
        let repo = result.repo.clone();

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
        let Some(result) = self.search.results.get(self.search.selected) else {
            return;
        };
        let repo = result.repo.clone();

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
            tokio::spawn(async move {
                let _ = client.unwatch_repo(&owner, &name).await;
            });
        } else {
            self.watched_repos.insert(repo);
            tokio::spawn(async move {
                let _ = client.watch_repo(&owner, &name).await;
            });
        }
    }

    fn handle_pr_browser(&mut self, key: KeyEvent) {
        // Tab switching always takes priority
        if key.modifiers == KeyModifiers::CONTROL {
            match key.code {
                KeyCode::Char('l') => { self.pr_tab = self.pr_tab.next(); return; }
                KeyCode::Char('h') => { self.pr_tab = self.pr_tab.prev(); return; }
                _ => {}
            }
        }
        match self.pr_tab {
            PrTab::All              => self.handle_pr_all(key),
            PrTab::ByPeople         => self.handle_pr_people(key),
            PrTab::ByRepo           => self.handle_pr_repo(key),
            PrTab::ReviewsRequested => self.handle_pr_reviews(key),
            PrTab::Watching         => self.handle_pr_watching(key),
        }
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
            (_, KeyCode::Enter) | (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
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
            (_, KeyCode::Enter) | (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
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
            (_, KeyCode::Enter) | (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
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
            (_, KeyCode::Enter) | (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
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
            (_, KeyCode::Enter) | (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
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

    fn open_diff(&mut self) {
        let Some(ref preview) = self.search.preview else { return; };
        let Some(commit) = preview.commits.get(self.preview_commit_selected) else { return; };
        let sha = commit.sha.clone();
        let Some(r) = self.search.results.get(self.search.selected) else { return; };
        let repo = r.repo.clone();

        self.diff_header = format!("{}  {}", repo, sha);
        self.diff_repo = repo.clone();
        self.diff_sha = sha.clone();
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
            (_, KeyCode::Char('q')) => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_picker(&mut self, key: KeyEvent) {
        if self.repo_prompt_active {
            match key.code {
                KeyCode::Esc => {
                    self.repo_prompt_active = false;
                    self.repo_prompt_input.clear();
                }
                KeyCode::Enter => self.fire_repo_question(),
                KeyCode::Backspace => { self.repo_prompt_input.pop(); }
                KeyCode::Char(c) => self.repo_prompt_input.push(c),
                _ => {}
            }
            return;
        }

        if key.code == KeyCode::Esc {
            match (&self.focus, self.picker_insert) {
                (Focus::Preview, _) => self.focus = Focus::Results,
                (Focus::Results, true) => self.picker_insert = false,
                (Focus::Results, false) => self.mode = Mode::Normal,
            }
            return;
        }

        match &self.focus {
            Focus::Results if self.picker_insert => match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                    if self.search.preview.as_ref().map(|p| !p.commits.is_empty()).unwrap_or(false) {
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
                    self.search.next();
                    self.on_selection_change();
                }
                (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                    self.search.prev();
                    self.on_selection_change();
                }
                (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                    if self.search.preview.as_ref().map(|p| !p.commits.is_empty()).unwrap_or(false) {
                        self.focus = Focus::Preview;
                        self.preview_commit_selected = 0;
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('/')) => {
                    if self.search.results.get(self.search.selected).is_some() {
                        self.repo_prompt_active = true;
                        self.repo_prompt_input.clear();
                        self.repo_answer.clear();
                        self.repo_answer_loading = false;
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('w')) => {
                    if let Some(result) = self.search.results.get(self.search.selected) {
                        let repo = result.repo.clone();
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
        }
    }

    fn handle_diff(&mut self, key: KeyEvent) {
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
                self.mode = Mode::Picker;
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
            (KeyModifiers::NONE, KeyCode::Char('/')) => {
                self.diff_prompt_active = true;
                self.diff_prompt_input.clear();
            }
            _ => {}
        }
    }

    fn fire_repo_question(&mut self) {
        let question = self.repo_prompt_input.trim().to_string();
        if question.is_empty() { return; }
        let Some(result) = self.search.results.get(self.search.selected) else { return; };
        let repo = result.repo.clone();
        let Some(ref client) = self.llm else { return; };
        let client = Arc::clone(client);
        let tx = self.repo_answer_tx.clone();
        self.repo_prompt_active = false;
        self.repo_prompt_input.clear();
        self.repo_answer.clear();
        self.repo_answer_loading = true;
        self.repo_progress.clear();
        let progress_tx = self.repo_progress_tx.clone();
        tokio::spawn(async move {
            let result = client.ask_repo(&repo, &question, progress_tx).await.map_err(|e| e.to_string());
            let _ = tx.send(result).await;
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

fn open_in_browser(url: &str) {
    #[cfg(target_os = "macos")]
    { std::process::Command::new("open").args(["-na", "Google Chrome", "--args", "--new-window", url]).spawn().ok(); }
    #[cfg(target_os = "linux")]
    { std::process::Command::new("xdg-open").arg(url).spawn().ok(); }
    #[cfg(target_os = "windows")]
    { std::process::Command::new("cmd").args(["/c", "start", url]).spawn().ok(); }
}
