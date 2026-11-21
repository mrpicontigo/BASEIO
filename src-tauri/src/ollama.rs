//! Ollama runtime integration over its local HTTP API.
//! Docs: https://github.com/ollama/ollama/blob/main/docs/api.md
//!
//! Safety: the only process this module ever starts is the Ollama executable
//! found at a known location, with the fixed argument `serve`. Model names are
//! validated with `catalog::is_valid_model_ref` and sent as JSON, never as
//! command-line arguments.

use crate::catalog::is_valid_model_ref;
use crate::error::{AppError, AppResult};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::watch;

pub fn base_url() -> String {
    match std::env::var("OLLAMA_HOST") {
        Ok(h) if !h.trim().is_empty() => {
            let h = h.trim().trim_end_matches('/');
            let h = h.replace("0.0.0.0", "127.0.0.1");
            if h.starts_with("http://") || h.starts_with("https://") { h } else { format!("http://{h}") }
        }
        _ => "http://127.0.0.1:11434".into(),
    }
}

/// Locate `ollama.exe`: default per-user install dir, then PATH.
pub fn find_executable() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "ollama.exe" } else { "ollama" };
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let p = PathBuf::from(local).join("Programs").join("Ollama").join(exe);
        if p.is_file() {
            return Some(p);
        }
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).map(|d| d.join(exe)).find(|p| p.is_file())
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeStatus {
    pub installed: bool,
    pub executable: Option<String>,
    pub running: bool,
    pub version: Option<String>,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledModel {
    pub name: String,
    pub size: u64,
    pub digest: String,
    pub modified_at: String,
    #[serde(default)]
    pub details: Option<ModelDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelDetails {
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub parameter_size: Option<String>,
    #[serde(default)]
    pub quantization_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningModel {
    pub name: String,
    pub size: u64,
    #[serde(default)]
    pub size_vram: u64,
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    /// Base64 images, only for models with the "vision" capability.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
}

/// What a model can do, as reported by Ollama's `/api/show`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ModelInfo {
    pub name: String,
    /// e.g. "completion", "vision", "thinking", "tools", "embedding".
    pub capabilities: Vec<String>,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
    pub context_length: Option<u64>,
}

impl ModelInfo {
    pub fn has(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }
}

/// Progress event streamed to the UI during a pull.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum PullEvent {
    Status { message: String },
    Progress { completed: u64, total: u64 },
    Verifying,
    Done,
}

/// Token event streamed to the UI during chat.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ChatEvent {
    /// Reasoning models (qwen3, deepseek-r1) think before answering.
    Thinking,
    Token { content: String },
    Done { eval_count: Option<u64>, eval_duration_ns: Option<u64> },
}

/// Splits a byte stream into complete newline-delimited JSON lines.
#[derive(Default)]
pub struct NdjsonBuffer {
    buf: Vec<u8>,
}

impl NdjsonBuffer {
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(chunk);
        let mut lines = Vec::new();
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=pos).collect();
            let s = String::from_utf8_lossy(&line).trim().to_string();
            if !s.is_empty() {
                lines.push(s);
            }
        }
        lines
    }
    pub fn finish(&mut self) -> Option<String> {
        let s = String::from_utf8_lossy(&self.buf).trim().to_string();
        self.buf.clear();
        (!s.is_empty()).then_some(s)
    }
}

/// Aggregates per-layer pull progress into one overall figure.
#[derive(Default)]
pub struct PullProgress {
    layers: HashMap<String, (u64, u64)>,
}

impl PullProgress {
    pub fn update(&mut self, digest: &str, completed: u64, total: u64) -> (u64, u64) {
        self.layers.insert(digest.to_string(), (completed.min(total), total));
        self.layers.values().fold((0, 0), |(c, t), (lc, lt)| (c + lc, t + lt))
    }
}

#[derive(Clone)]
pub struct Ollama {
    http: reqwest::Client,
    base: String,
}

fn net_err(e: reqwest::Error) -> AppError {
    if e.is_connect() {
        AppError::RuntimeUnavailable("Ollama is not running. Start it from the Models page.".into())
    } else if e.is_timeout() {
        AppError::Network("Ollama did not respond in time".into())
    } else {
        AppError::Network(e.to_string())
    }
}

async fn check(resp: reqwest::Response) -> AppResult<reqwest::Response> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    let msg = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
        .unwrap_or(body);
    if status == reqwest::StatusCode::NOT_FOUND {
        Err(AppError::NotFound(msg))
    } else {
        Err(AppError::Runtime(format!("Ollama error ({status}): {msg}")))
    }
}

fn validate(model: &str) -> AppResult<()> {
    if is_valid_model_ref(model) {
        Ok(())
    } else {
        Err(AppError::Validation(format!("'{model}' is not a valid model name")))
    }
}

impl Ollama {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .build()
            .expect("http client");
        Ollama { http, base: base_url() }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    pub async fn version(&self) -> AppResult<String> {
        #[derive(Deserialize)]
        struct V { version: String }
        let r = self.http.get(self.url("/api/version")).timeout(Duration::from_secs(3)).send().await.map_err(net_err)?;
        Ok(check(r).await?.json::<V>().await.map_err(net_err)?.version)
    }

    pub async fn status(&self) -> RuntimeStatus {
        let exe = find_executable();
        let version = self.version().await.ok();
        RuntimeStatus {
            installed: exe.is_some() || version.is_some(),
            executable: exe.map(|p| p.to_string_lossy().to_string()),
            running: version.is_some(),
            version,
            base_url: self.base.clone(),
        }
    }

    /// Start `ollama serve` in the background and wait until it answers.
    pub async fn start(&self) -> AppResult<RuntimeStatus> {
        if self.version().await.is_ok() {
            return Ok(self.status().await);
        }
        let exe = find_executable().ok_or_else(|| {
            AppError::RuntimeUnavailable("Ollama is not installed. Install it from https://ollama.com/download".into())
        })?;
        let mut cmd = std::process::Command::new(&exe);
        cmd.arg("serve")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            const DETACHED_PROCESS: u32 = 0x0000_0008;
            cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
        }
        cmd.spawn().map_err(|e| AppError::Runtime(format!("could not start Ollama: {e}")))?;

        for _ in 0..40 {
            tokio::time::sleep(Duration::from_millis(250)).await;
            if self.version().await.is_ok() {
                return Ok(self.status().await);
            }
        }
        Err(AppError::RuntimeUnavailable("Ollama was started but did not respond within 10 seconds".into()))
    }

    pub async fn list(&self) -> AppResult<Vec<InstalledModel>> {
        #[derive(Deserialize)]
        struct R { models: Vec<InstalledModel> }
        let r = self.http.get(self.url("/api/tags")).timeout(Duration::from_secs(10)).send().await.map_err(net_err)?;
        Ok(check(r).await?.json::<R>().await.map_err(net_err)?.models)
    }

    pub async fn running(&self) -> AppResult<Vec<RunningModel>> {
        #[derive(Deserialize)]
        struct R { models: Vec<RunningModel> }
        let r = self.http.get(self.url("/api/ps")).timeout(Duration::from_secs(10)).send().await.map_err(net_err)?;
        Ok(check(r).await?.json::<R>().await.map_err(net_err)?.models)
    }

    pub async fn show(&self, model: &str) -> AppResult<ModelInfo> {
        validate(model)?;
        let r = self
            .http
            .post(self.url("/api/show"))
            .json(&serde_json::json!({ "model": model }))
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .map_err(net_err)?;
        let v: serde_json::Value = check(r).await?.json().await.map_err(net_err)?;
        Ok(parse_show(model, &v))
    }

    pub async fn is_installed(&self, model: &str) -> AppResult<bool> {
        let want = normalize_ref(model);
        Ok(self.list().await?.iter().any(|m| normalize_ref(&m.name) == want))
    }

    /// Pull a model, streaming progress. Ollama verifies each layer's sha256
    /// digest itself; afterwards we confirm the model is actually listed.
    pub async fn pull(
        &self,
        model: &str,
        mut cancel: watch::Receiver<bool>,
        mut on_event: impl FnMut(PullEvent),
    ) -> AppResult<()> {
        validate(model)?;
        let resp = self
            .http
            .post(self.url("/api/pull"))
            .json(&serde_json::json!({ "model": model, "stream": true }))
            .send()
            .await
            .map_err(net_err)?;
        let resp = check(resp).await?;
        let mut stream = resp.bytes_stream();
        let mut buf = NdjsonBuffer::default();
        let mut progress = PullProgress::default();
        let mut success = false;

        loop {
            let chunk = tokio::select! {
                _ = cancel.changed() => return Err(AppError::Cancelled),
                c = tokio::time::timeout(Duration::from_secs(120), stream.next()) => match c {
                    Err(_) => return Err(AppError::Network("Download stalled: no data for 2 minutes".into())),
                    Ok(None) => break,
                    Ok(Some(Err(e))) => return Err(AppError::Network(format!("Download interrupted: {e}"))),
                    Ok(Some(Ok(b))) => b,
                },
            };
            for line in buf.push(&chunk) {
                success |= handle_pull_line(&line, &mut progress, &mut on_event)?;
            }
        }
        if let Some(line) = buf.finish() {
            success |= handle_pull_line(&line, &mut progress, &mut on_event)?;
        }
        if !success {
            return Err(AppError::Network("Download ended before Ollama reported success".into()));
        }
        on_event(PullEvent::Verifying);
        if !self.is_installed(model).await? {
            return Err(AppError::Runtime(format!("Ollama reported success but '{model}' is not in the installed list")));
        }
        on_event(PullEvent::Done);
        Ok(())
    }

    pub async fn delete(&self, model: &str) -> AppResult<()> {
        validate(model)?;
        let r = self
            .http
            .delete(self.url("/api/delete"))
            .json(&serde_json::json!({ "model": model }))
            .send()
            .await
            .map_err(net_err)?;
        check(r).await?;
        Ok(())
    }

    /// Load a model into memory (empty generate request).
    pub async fn load(&self, model: &str) -> AppResult<()> {
        self.keep_alive(model, serde_json::json!("10m")).await
    }

    /// Unload a model from memory immediately.
    pub async fn unload(&self, model: &str) -> AppResult<()> {
        self.keep_alive(model, serde_json::json!(0)).await
    }

    async fn keep_alive(&self, model: &str, keep: serde_json::Value) -> AppResult<()> {
        validate(model)?;
        let r = self
            .http
            .post(self.url("/api/generate"))
            .json(&serde_json::json!({ "model": model, "keep_alive": keep, "stream": false }))
            .timeout(Duration::from_secs(300))
            .send()
            .await
            .map_err(net_err)?;
        check(r).await?;
        Ok(())
    }

    /// Stream a chat completion. Returns the full assistant reply.
    /// `num_ctx` raises the context window (e.g. when documents are attached).
    pub async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        num_ctx: Option<u64>,
        mut cancel: watch::Receiver<bool>,
        mut on_event: impl FnMut(ChatEvent),
    ) -> AppResult<String> {
        validate(model)?;
        let mut body = serde_json::json!({ "model": model, "messages": messages, "stream": true });
        if let Some(n) = num_ctx {
            body["options"] = serde_json::json!({ "num_ctx": n });
        }
        let resp = self
            .http
            .post(self.url("/api/chat"))
            .json(&body)
            .send()
            .await
            .map_err(net_err)?;
        let resp = check(resp).await?;
        let mut stream = resp.bytes_stream();
        let mut buf = NdjsonBuffer::default();
        let mut full = String::new();

        loop {
            let chunk = tokio::select! {
                _ = cancel.changed() => return Err(AppError::Cancelled),
                // Generous: loading a large model from disk can take minutes.
                c = tokio::time::timeout(Duration::from_secs(300), stream.next()) => match c {
                    Err(_) => return Err(AppError::Network("The model stopped responding".into())),
                    Ok(None) => break,
                    Ok(Some(Err(e))) => return Err(AppError::Network(e.to_string())),
                    Ok(Some(Ok(b))) => b,
                },
            };
            for line in buf.push(&chunk) {
                let v: serde_json::Value = serde_json::from_str(&line)
                    .map_err(|e| AppError::Runtime(format!("bad response from Ollama: {e}")))?;
                if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
                    return Err(AppError::Runtime(err.to_string()));
                }
                if v.pointer("/message/thinking").and_then(|c| c.as_str()).is_some_and(|t| !t.is_empty()) && full.is_empty() {
                    on_event(ChatEvent::Thinking);
                }
                if let Some(c) = v.pointer("/message/content").and_then(|c| c.as_str()) {
                    if !c.is_empty() {
                        full.push_str(c);
                        on_event(ChatEvent::Token { content: c.to_string() });
                    }
                }
                if v.get("done").and_then(|d| d.as_bool()) == Some(true) {
                    on_event(ChatEvent::Done {
                        eval_count: v.get("eval_count").and_then(|x| x.as_u64()),
                        eval_duration_ns: v.get("eval_duration").and_then(|x| x.as_u64()),
                    });
                    return Ok(full);
                }
            }
        }
        Err(AppError::Network("The reply ended unexpectedly".into()))
    }
}

/// Returns true when the line reports overall success.
fn handle_pull_line(line: &str, progress: &mut PullProgress, on_event: &mut impl FnMut(PullEvent)) -> AppResult<bool> {
    let v: serde_json::Value =
        serde_json::from_str(line).map_err(|e| AppError::Runtime(format!("bad response from Ollama: {e}")))?;
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        return Err(AppError::Runtime(explain_pull_error(err)));
    }
    let status = v.get("status").and_then(|s| s.as_str()).unwrap_or_default();
    if status == "success" {
        return Ok(true);
    }
    match (v.get("digest").and_then(|d| d.as_str()), v.get("total").and_then(|t| t.as_u64())) {
        (Some(d), Some(total)) => {
            let completed = v.get("completed").and_then(|c| c.as_u64()).unwrap_or(0);
            let (c, t) = progress.update(d, completed, total);
            on_event(PullEvent::Progress { completed: c, total: t });
        }
        _ if !status.is_empty() => on_event(PullEvent::Status { message: status.to_string() }),
        _ => {}
    }
    Ok(false)
}

fn explain_pull_error(err: &str) -> String {
    let e = err.to_lowercase();
    if e.contains("file does not exist") || e.contains("manifest unknown") {
        format!("This model or tag does not exist in the registry ({err})")
    } else if e.contains("401") || e.contains("403") || e.contains("unauthorized") || e.contains("gated") {
        format!("Access denied. This model may be gated and require accepting its licence on the publisher's site ({err})")
    } else if e.contains("no space") {
        format!("Not enough disk space ({err})")
    } else {
        err.to_string()
    }
}

fn parse_show(model: &str, v: &serde_json::Value) -> ModelInfo {
    let s = |p: &str| v.pointer(p).and_then(|x| x.as_str()).map(String::from);
    let context_length = v.get("model_info").and_then(|mi| mi.as_object()).and_then(|o| {
        o.iter().find(|(k, _)| k.ends_with(".context_length")).and_then(|(_, x)| x.as_u64())
    });
    ModelInfo {
        name: model.to_string(),
        capabilities: v
            .get("capabilities")
            .and_then(|c| c.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        family: s("/details/family"),
        parameter_size: s("/details/parameter_size"),
        quantization_level: s("/details/quantization_level"),
        context_length,
    }
}

/// `llama3.2` and `llama3.2:latest` refer to the same model.
pub fn normalize_ref(s: &str) -> String {
    let s = s.trim().to_lowercase();
    if s.rsplit_once(':').map_or(true, |(_, t)| t.contains('/')) {
        format!("{s}:latest")
    } else {
        s
    }
}

/// Exact download size from the public registry manifest (sum of layers).
/// Only for Ollama-library models (`name:tag` or `namespace/name:tag`).
pub async fn registry_size(http: &reqwest::Client, model: &str) -> AppResult<u64> {
    validate(model)?;
    if model.starts_with("hf.co/") || model.starts_with("huggingface.co/") {
        return Err(AppError::Validation("registry size lookup is only for Ollama library models".into()));
    }
    let (name, tag) = model.rsplit_once(':').unwrap_or((model, "latest"));
    let name = if name.contains('/') { name.to_string() } else { format!("library/{name}") };
    let url = format!("https://registry.ollama.ai/v2/{name}/manifests/{tag}");
    #[derive(Deserialize)]
    struct Layer { size: u64 }
    #[derive(Deserialize)]
    struct Manifest { config: Layer, layers: Vec<Layer> }
    let r = http
        .get(url)
        .header("Accept", "application/vnd.docker.distribution.manifest.v2+json")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| AppError::Network(format!("could not reach the model registry: {e}")))?;
    let m: Manifest = check(r).await?.json().await.map_err(|e| AppError::Network(e.to_string()))?;
    Ok(m.config.size + m.layers.iter().map(|l| l.size).sum::<u64>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ndjson_split_across_chunks() {
        let mut b = NdjsonBuffer::default();
        assert!(b.push(b"{\"a\":").is_empty());
        assert_eq!(b.push(b"1}\n{\"b\":2}\n{\"c\""), vec!["{\"a\":1}", "{\"b\":2}"]);
        assert_eq!(b.push(b":3}"), Vec::<String>::new());
        assert_eq!(b.finish().as_deref(), Some("{\"c\":3}"));
    }

    #[test]
    fn progress_aggregates_layers() {
        let mut p = PullProgress::default();
        assert_eq!(p.update("a", 50, 100), (50, 100));
        assert_eq!(p.update("b", 10, 20), (60, 120));
        assert_eq!(p.update("a", 100, 100), (110, 120));
    }

    #[test]
    fn pull_lines() {
        let mut p = PullProgress::default();
        let mut events = Vec::new();
        let mut f = |e: PullEvent| events.push(e);
        assert!(!handle_pull_line(r#"{"status":"pulling manifest"}"#, &mut p, &mut f).unwrap());
        assert!(!handle_pull_line(r#"{"status":"pulling x","digest":"sha256:1","total":10,"completed":5}"#, &mut p, &mut f).unwrap());
        assert!(handle_pull_line(r#"{"status":"success"}"#, &mut p, &mut f).unwrap());
        assert!(handle_pull_line(r#"{"error":"pull model manifest: file does not exist"}"#, &mut p, &mut f).is_err());
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn show_parsing() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"capabilities":["completion","vision"],"details":{"family":"gemma3","parameter_size":"4.3B"},
                "model_info":{"gemma3.context_length":131072,"general.architecture":"gemma3"}}"#,
        )
        .unwrap();
        let i = parse_show("gemma3:4b", &v);
        assert!(i.has("vision"));
        assert_eq!(i.context_length, Some(131072));
        assert_eq!(i.parameter_size.as_deref(), Some("4.3B"));
    }

    #[test]
    fn images_omitted_when_empty() {
        let m = ChatMessage { role: "user".into(), content: "hi".into(), images: vec![] };
        assert!(!serde_json::to_string(&m).unwrap().contains("images"));
    }

    #[test]
    fn normalize() {
        assert_eq!(normalize_ref("llama3.2"), "llama3.2:latest");
        assert_eq!(normalize_ref("Llama3.2:3B"), "llama3.2:3b");
    }
}
