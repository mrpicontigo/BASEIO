//! Tauri command surface. Keep commands thin: validate input, delegate to a
//! module, map errors to `AppError`.

use crate::attachments::{self, Attachment, AttachmentKind};
use crate::catalog::{self, is_valid_model_ref, Catalog, License};
use crate::db::{Conversation, Message};
use crate::error::{AppError, AppResult};
use crate::hardware::{self, HardwareInfo};
use crate::ollama::{self, ChatEvent, ChatMessage, InstalledModel, ModelInfo, PullEvent, RunningModel, RuntimeStatus};
use crate::recommend::{self, Budget, Compatibility, Recommendation};
use crate::state::AppState;
use serde::Serialize;
use std::path::PathBuf;
use tauri::ipc::Channel;
use tauri::State;

/// Where Ollama stores models: `OLLAMA_MODELS` or `%USERPROFILE%\.ollama\models`.
pub fn ollama_models_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("OLLAMA_MODELS") {
        if !p.trim().is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|h| PathBuf::from(h).join(".ollama").join("models"))
}

pub(crate) async fn detect_hw() -> AppResult<HardwareInfo> {
    tauri::async_runtime::spawn_blocking(hardware::detect)
        .await
        .map_err(|e| AppError::Hardware(format!("detection task failed: {e}")))
}

async fn budget() -> AppResult<Budget> {
    let hw = detect_hw().await?;
    Ok(Budget::from_hardware(&hw, ollama_models_dir().as_deref()))
}

// ---------------------------------------------------------------- hardware

#[tauri::command]
pub async fn get_hardware_info() -> AppResult<HardwareInfo> {
    detect_hw().await
}

// ---------------------------------------------------------------- catalogue

#[tauri::command]
pub fn get_catalog() -> Catalog {
    catalog::curated().clone()
}

#[derive(Serialize)]
pub struct CatalogAssessment {
    pub models_dir: Option<String>,
    pub disk_free_bytes: Option<u64>,
    pub variants: Vec<Compatibility>,
}

/// Compatibility of every catalogue variant with this machine.
#[tauri::command]
pub async fn assess_catalog() -> AppResult<CatalogAssessment> {
    let b = budget().await?;
    let variants = catalog::curated()
        .models
        .iter()
        .flat_map(|m| m.variants.iter())
        .map(|v| recommend::assess(v, &b))
        .collect();
    Ok(CatalogAssessment {
        models_dir: ollama_models_dir().map(|d| d.to_string_lossy().to_string()),
        disk_free_bytes: b.disk_free,
        variants,
    })
}

#[tauri::command]
pub async fn get_recommendations(use_case: Option<String>) -> AppResult<Vec<Recommendation>> {
    let b = budget().await?;
    Ok(recommend::recommend(catalog::curated(), &b, use_case.as_deref()))
}

// ---------------------------------------------------------------- runtime

#[tauri::command]
pub async fn runtime_status(state: State<'_, AppState>) -> AppResult<RuntimeStatus> {
    Ok(state.ollama.status().await)
}

#[tauri::command]
pub async fn start_runtime(state: State<'_, AppState>) -> AppResult<RuntimeStatus> {
    state.ollama.start().await
}

#[tauri::command]
pub async fn list_installed(state: State<'_, AppState>) -> AppResult<Vec<InstalledModel>> {
    state.ollama.list().await
}

#[tauri::command]
pub async fn list_running(state: State<'_, AppState>) -> AppResult<Vec<RunningModel>> {
    state.ollama.running().await
}

#[tauri::command]
pub async fn load_model(state: State<'_, AppState>, model: String) -> AppResult<()> {
    state.ollama.load(&model).await
}

#[tauri::command]
pub async fn unload_model(state: State<'_, AppState>, model: String) -> AppResult<()> {
    state.ollama.unload(&model).await
}

#[tauri::command]
pub async fn delete_model(state: State<'_, AppState>, model: String) -> AppResult<()> {
    if state.installs.active().iter().any(|k| k == &model) {
        return Err(AppError::Validation("Cancel the download before deleting this model".into()));
    }
    // Unload first so Windows doesn't keep the blob files locked.
    let _ = state.ollama.unload(&model).await;
    state.ollama.delete(&model).await
}

// ---------------------------------------------------------------- install

#[derive(Serialize)]
pub struct InstallPlan {
    pub tag: String,
    pub model_name: Option<String>,
    pub in_catalog: bool,
    /// Exact size from the registry manifest, when it could be fetched.
    pub verified_size_bytes: Option<u64>,
    /// Catalogue estimate (used only if verification failed).
    pub approx_size_bytes: Option<u64>,
    pub size_error: Option<String>,
    pub license: License,
    /// Key used to remember acceptance of this licence.
    pub license_key: String,
    pub license_accepted: bool,
    pub compatibility: Option<Compatibility>,
    pub disk_free_bytes: Option<u64>,
    pub already_installed: bool,
    pub runtime_running: bool,
    /// Hard blockers. Install is refused while this is non-empty.
    pub blockers: Vec<String>,
}

fn license_for(tag: &str) -> (License, String, Option<String>, Option<u64>, Option<&'static catalog::ModelVariant>) {
    match catalog::curated().find_variant(tag) {
        Some((m, v)) => {
            let lic = v.license.clone().unwrap_or_else(|| m.license.clone());
            let key = lic.name.clone();
            (lic, key, Some(m.name.clone()), Some(v.approx_size_bytes), Some(v))
        }
        None => (
            License {
                name: "Unknown — check the model page".into(),
                url: if tag.starts_with("hf.co/") {
                    format!("https://{}", tag.split(':').next().unwrap_or(tag))
                } else {
                    format!("https://ollama.com/library/{}", tag.split(':').next().unwrap_or(tag))
                },
                commercial_use: "unknown".into(),
                gated: false,
            },
            format!("custom:{tag}"),
            None,
            None,
            None,
        ),
    }
}

/// Everything the user needs to approve an install. Performs no changes.
#[tauri::command]
pub async fn prepare_install(state: State<'_, AppState>, tag: String) -> AppResult<InstallPlan> {
    if !is_valid_model_ref(&tag) {
        return Err(AppError::Validation(format!("'{tag}' is not a valid model name")));
    }
    let (license, license_key, model_name, approx, variant) = license_for(&tag);
    let b = budget().await?;

    let (verified, size_error) = if tag.starts_with("hf.co/") {
        (None, Some("Size is not known in advance for Hugging Face models".to_string()))
    } else {
        match ollama::registry_size(&state.http, &tag).await {
            Ok(s) => (Some(s), None),
            Err(AppError::NotFound(_)) => (None, Some("This model/tag was not found in the Ollama registry".into())),
            Err(e) => (None, Some(format!("Could not verify size (offline?): {e}"))),
        }
    };

    let compatibility = variant.map(|v| {
        let mut v = v.clone();
        if let Some(s) = verified {
            v.approx_size_bytes = s;
        }
        recommend::assess(&v, &b)
    });

    let status = state.ollama.status().await;
    let already_installed = if status.running { state.ollama.is_installed(&tag).await.unwrap_or(false) } else { false };
    let license_accepted = state.db.license_accepted(&license_key)?;

    let mut blockers = Vec::new();
    if !status.installed {
        blockers.push("Ollama is not installed".into());
    } else if !status.running {
        blockers.push("Ollama is not running".into());
    }
    if already_installed {
        blockers.push("This model is already installed".into());
    }
    if let (Some(size), Some(free)) = (verified.or(approx), b.disk_free) {
        if size + (1 << 30) > free {
            blockers.push("Not enough free disk space".into());
        }
    }
    if size_error.as_deref().is_some_and(|e| e.contains("not found")) {
        blockers.push("Model not found in the registry".into());
    }

    Ok(InstallPlan {
        tag,
        model_name,
        in_catalog: variant.is_some(),
        verified_size_bytes: verified,
        approx_size_bytes: approx,
        size_error,
        license,
        license_key,
        license_accepted,
        compatibility,
        disk_free_bytes: b.disk_free,
        already_installed,
        runtime_running: status.running,
        blockers,
    })
}

/// Download and install a model. Requires explicit licence acceptance.
#[tauri::command]
pub async fn install_model(
    state: State<'_, AppState>,
    tag: String,
    accept_license: bool,
    on_event: Channel<PullEvent>,
) -> AppResult<()> {
    if !is_valid_model_ref(&tag) {
        return Err(AppError::Validation(format!("'{tag}' is not a valid model name")));
    }
    let (_, license_key, ..) = license_for(&tag);
    if !accept_license && !state.db.license_accepted(&license_key)? {
        return Err(AppError::Validation("You must accept the model licence before installing".into()));
    }
    // Re-check disk space at install time; the plan may be stale.
    let b = budget().await?;
    if let (Some((_, v)), Some(free)) = (catalog::curated().find_variant(&tag), b.disk_free) {
        if v.approx_size_bytes + (1 << 30) > free {
            return Err(AppError::Validation("Not enough free disk space".into()));
        }
    }

    let cancel = state
        .installs
        .start(&tag)
        .ok_or_else(|| AppError::Validation(format!("'{tag}' is already downloading")))?;
    if accept_license {
        state.db.accept_license(&license_key, &tag)?;
    }
    let result = state
        .ollama
        .pull(&tag, cancel, |e| {
            let _ = on_event.send(e);
        })
        .await;
    state.installs.finish(&tag);
    result
}

#[tauri::command]
pub fn cancel_install(state: State<'_, AppState>, tag: String) -> bool {
    state.installs.cancel(&tag)
}

#[tauri::command]
pub fn active_installs(state: State<'_, AppState>) -> Vec<String> {
    state.installs.active()
}

// ---------------------------------------------------------------- chat

#[tauri::command]
pub fn list_conversations(state: State<'_, AppState>) -> AppResult<Vec<Conversation>> {
    state.db.list_conversations()
}

#[tauri::command]
pub fn create_conversation(state: State<'_, AppState>, model: String) -> AppResult<Conversation> {
    if !is_valid_model_ref(&model) {
        return Err(AppError::Validation(format!("'{model}' is not a valid model name")));
    }
    state.db.create_conversation(&model, "New chat")
}

#[tauri::command]
pub fn get_messages(state: State<'_, AppState>, conversation_id: i64) -> AppResult<Vec<Message>> {
    state.db.messages(conversation_id)
}

#[tauri::command]
pub fn delete_conversation(state: State<'_, AppState>, conversation_id: i64) -> AppResult<()> {
    state.db.delete_conversation(conversation_id)
}

fn title_from(text: &str) -> String {
    let t: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > 48 {
        format!("{}…", t.chars().take(48).collect::<String>())
    } else {
        t
    }
}

/// Persist the user message, stream the reply, persist the reply.
/// On cancel, whatever was generated so far is kept.
#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    conversation_id: i64,
    content: String,
    attachments: Option<Vec<Attachment>>,
    on_event: Channel<ChatEvent>,
) -> AppResult<Message> {
    let content = content.trim().to_string();
    let attachments = attachments.unwrap_or_default();
    if content.is_empty() && attachments.is_empty() {
        return Err(AppError::Validation("Message is empty".into()));
    }
    if attachments.len() > 10 {
        return Err(AppError::Validation("Attach at most 10 files per message".into()));
    }
    // The UI got these from `read_attachment`, but enforce limits again.
    for a in &attachments {
        let len = a.text.as_ref().map_or(0, |t| t.chars().count()) + a.data_base64.as_ref().map_or(0, |d| d.len() / 2);
        if len > 15 * 1024 * 1024 {
            return Err(AppError::Validation(format!("'{}' is too large", a.name)));
        }
    }
    let conv = state
        .db
        .get_conversation(conversation_id)?
        .ok_or_else(|| AppError::NotFound("Conversation not found".into()))?;

    let info = state.ollama.show(&conv.model).await.map_err(|e| match e {
        AppError::NotFound(_) => AppError::NotFound(format!("{} is not installed any more. Pick another model.", conv.model)),
        other => other,
    })?;
    let vision = info.has("vision");
    if !vision && attachments.iter().any(|a| a.kind == AttachmentKind::Image) {
        return Err(AppError::Validation(format!(
            "{} can't see images. Switch to a vision model (e.g. gemma3:4b or llava:7b), or remove the image.",
            conv.model
        )));
    }

    let key = conversation_id.to_string();
    let cancel = state
        .chats
        .start(&key)
        .ok_or_else(|| AppError::Validation("A reply is already being generated".into()))?;

    let run = async {
        let history = state.db.messages(conversation_id)?;
        if history.is_empty() {
            let t = if content.is_empty() { attachments[0].name.clone() } else { content.clone() };
            state.db.rename_conversation(conversation_id, &title_from(&t))?;
        }
        state.db.add_message_with(conversation_id, "user", &content, &attachments)?;

        let to_msg = |role: String, text: &str, atts: &[Attachment]| {
            let ctx = attachments::as_prompt_context(atts);
            ChatMessage {
                role,
                content: if ctx.is_empty() { text.to_string() } else { format!("{ctx}\n{text}") },
                images: if vision {
                    atts.iter().filter_map(|a| a.data_base64.clone()).collect()
                } else {
                    Vec::new()
                },
            }
        };
        let mut msgs: Vec<ChatMessage> = history.iter().map(|m| to_msg(m.role.clone(), &m.content, &m.attachments)).collect();
        msgs.push(to_msg("user".into(), &content, &attachments));

        // Documents need a bigger context window than Ollama's default.
        let has_docs = attachments.iter().chain(history.iter().flat_map(|m| m.attachments.iter()))
            .any(|a| a.kind == AttachmentKind::Text);
        let num_ctx = has_docs.then(|| info.context_length.unwrap_or(8192).min(16384));

        let mut partial = String::new();
        let res = state
            .ollama
            .chat(&conv.model, &msgs, num_ctx, cancel, |e| {
                if let ChatEvent::Token { content } = &e {
                    partial.push_str(content);
                }
                let _ = on_event.send(e);
            })
            .await;
        match res {
            Ok(full) => state.db.add_message(conversation_id, "assistant", &full),
            Err(AppError::Cancelled) if !partial.is_empty() => {
                state.db.add_message(conversation_id, "assistant", &partial)
            }
            Err(e) => Err(e),
        }
    };
    let out = run.await;
    state.chats.finish(&key);
    out
}

#[tauri::command]
pub fn cancel_chat(state: State<'_, AppState>, conversation_id: i64) -> bool {
    state.chats.cancel(&conversation_id.to_string())
}

#[tauri::command]
pub fn set_conversation_model(state: State<'_, AppState>, conversation_id: i64, model: String) -> AppResult<()> {
    if !is_valid_model_ref(&model) {
        return Err(AppError::Validation(format!("'{model}' is not a valid model name")));
    }
    state.db.set_conversation_model(conversation_id, &model)
}

/// Capabilities of an installed model (text / vision / thinking …).
#[tauri::command]
pub async fn model_info(state: State<'_, AppState>, model: String) -> AppResult<ModelInfo> {
    state.ollama.show(&model).await
}

/// Read a file the user picked or dropped. Runs off the UI thread (PDFs can be slow).
#[tauri::command]
pub async fn read_attachment(path: String) -> AppResult<Attachment> {
    tauri::async_runtime::spawn_blocking(move || attachments::read(std::path::Path::new(&path)))
        .await
        .map_err(|e| AppError::Internal(format!("reading file failed: {e}")))?
}

#[tauri::command]
pub fn attachment_extensions() -> Vec<&'static str> {
    attachments::supported_extensions()
}

// ---------------------------------------------------------------- natural language

#[derive(Serialize)]
pub struct RequestInterpretation {
    pub intent: crate::intent::Intent,
    pub recommendations: Vec<Recommendation>,
}

/// Turn a plain-language request into recommendations. Never installs anything.
#[tauri::command]
pub async fn interpret_request(text: String) -> AppResult<RequestInterpretation> {
    if text.trim().is_empty() {
        return Err(AppError::Validation("Describe what you want to do".into()));
    }
    if text.len() > 2000 {
        return Err(AppError::Validation("Please keep the request under 2000 characters".into()));
    }
    let cat = catalog::curated();
    let intent = crate::intent::parse(&text, cat);
    let b = budget().await?;
    let recommendations = crate::intent::recommend_for(&intent, cat, &b);
    Ok(RequestInterpretation { intent, recommendations })
}
