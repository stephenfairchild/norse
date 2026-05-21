use tokio::sync::mpsc;
use crate::github::RepoPreview;

pub struct SearchResult {
    pub repo: String,
}

pub struct SearchState {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected: usize,
    pub loading: bool,
    pub error: Option<String>,
    pub results_changed: bool,
    pub preview: Option<RepoPreview>,
    pub preview_loading: bool,
    pub tx: mpsc::Sender<Result<Vec<SearchResult>, String>>,
    pub rx: mpsc::Receiver<Result<Vec<SearchResult>, String>>,
    pub preview_tx: mpsc::Sender<(String, Result<RepoPreview, String>)>,
    pub preview_rx: mpsc::Receiver<(String, Result<RepoPreview, String>)>,
}

impl SearchState {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(16);
        let (preview_tx, preview_rx) = mpsc::channel(8);
        Self {
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            loading: false,
            error: None,
            results_changed: false,
            preview: None,
            preview_loading: false,
            tx,
            rx,
            preview_tx,
            preview_rx,
        }
    }

    pub fn reset(&mut self) {
        self.query.clear();
        self.results.clear();
        self.selected = 0;
        self.loading = false;
        self.error = None;
        self.results_changed = false;
        self.preview = None;
        self.preview_loading = false;
    }

    pub fn poll(&mut self) {
        let mut latest = None;
        while let Ok(msg) = self.rx.try_recv() {
            latest = Some(msg);
        }
        if let Some(msg) = latest {
            self.loading = false;
            self.results_changed = true;
            match msg {
                Ok(results) => {
                    self.results = results;
                    self.selected = 0;
                    self.error = None;
                }
                Err(e) => {
                    self.error = Some(e);
                    self.results.clear();
                }
            }
        }
    }


}
