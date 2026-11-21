use crate::db::Db;
use crate::ollama::Ollama;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::watch;

/// Tracks cancellable long-running jobs (downloads, chat replies) by key.
#[derive(Default)]
pub struct Jobs(Mutex<HashMap<String, watch::Sender<bool>>>);

impl Jobs {
    /// Register a job; returns `None` if a job with this key is already running.
    pub fn start(&self, key: &str) -> Option<watch::Receiver<bool>> {
        let mut m = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if m.contains_key(key) {
            return None;
        }
        let (tx, rx) = watch::channel(false);
        m.insert(key.to_string(), tx);
        Some(rx)
    }

    pub fn cancel(&self, key: &str) -> bool {
        let m = self.0.lock().unwrap_or_else(|p| p.into_inner());
        m.get(key).map(|tx| tx.send(true).is_ok()).unwrap_or(false)
    }

    pub fn finish(&self, key: &str) {
        self.0.lock().unwrap_or_else(|p| p.into_inner()).remove(key);
    }

    pub fn active(&self) -> Vec<String> {
        self.0.lock().unwrap_or_else(|p| p.into_inner()).keys().cloned().collect()
    }
}

pub struct AppState {
    pub ollama: Ollama,
    pub http: reqwest::Client,
    pub db: Db,
    pub installs: Jobs,
    pub chats: Jobs,
    pub image_gen: Jobs,
    pub image_paths: crate::imagegen::Paths,
}
