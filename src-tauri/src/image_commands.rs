//! Tauri commands for local image generation (see imagegen.rs).

use crate::catalog::License;
use crate::commands::detect_hw;
use crate::db::GeneratedImage;
use crate::error::{AppError, AppResult};
use crate::imagegen::{self, GenEvent, GenRequest, ImageFit, ImageModel, InstallEvent};
use crate::recommend::{disk_free_for, Tier};
use crate::state::AppState;
use base64::Engine;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;

#[derive(Serialize)]
pub struct ImageModelStatus {
    pub model: ImageModel,
    pub installed: bool,
    pub download_bytes: u64,
    pub fit: ImageFit,
}

#[derive(Serialize)]
pub struct ImageOverview {
    pub engine_installed: bool,
    pub engine_variant: Option<String>,
    /// Size of the engine download if it is not installed yet.
    pub engine_download_bytes: u64,
    pub engine_license: License,
    pub models: Vec<ImageModelStatus>,
}

fn license_key(m: &ImageModel) -> String {
    format!("image:{}", m.license.name)
}

#[tauri::command]
pub async fn image_overview(state: State<'_, AppState>) -> AppResult<ImageOverview> {
    let hw = detect_hw().await?;
    let p = &state.image_paths;
    let disk = disk_free_for(&hw, &p.root);
    let engine = p.engine_exe();
    let variant = imagegen::preferred_variant(&hw);
    let models = imagegen::catalog()
        .models
        .iter()
        .map(|m| {
            let installed = p.model_installed(m);
            let download_bytes = m.files.iter()
                .filter(|f| !crate::download::present(&p.model_file(m, f), f.size))
                .map(|f| f.size).sum();
            ImageModelStatus { model: m.clone(), installed, download_bytes, fit: imagegen::assess(m, &hw, disk) }
        })
        .collect();
    Ok(ImageOverview {
        engine_installed: engine.is_some(),
        engine_variant: engine.map(|(v, _)| v),
        engine_download_bytes: if p.engine_exe().is_some() { 0 } else { imagegen::catalog().engine.builds[variant].size },
        engine_license: imagegen::catalog().engine.license.clone(),
        models,
    })
}

#[tauri::command]
pub fn image_license_accepted(state: State<'_, AppState>, model_id: String) -> AppResult<bool> {
    let m = imagegen::catalog().find(&model_id)?;
    state.db.license_accepted(&license_key(m))
}

#[tauri::command]
pub async fn install_image_model(
    state: State<'_, AppState>,
    model_id: String,
    accept_license: bool,
    on_event: Channel<InstallEvent>,
) -> AppResult<()> {
    let m = imagegen::catalog().find(&model_id)?.clone();
    let key = license_key(&m);
    if !accept_license && !state.db.license_accepted(&key)? {
        return Err(AppError::Validation("You must accept the model licence before installing".into()));
    }
    let hw = detect_hw().await?;
    if let Some(free) = disk_free_for(&hw, &state.image_paths.root) {
        if free < m.total_size() + (1 << 30) {
            return Err(AppError::Validation("Not enough free disk space".into()));
        }
    }
    let job = format!("img-install:{model_id}");
    let cancel = state.installs.start(&job).ok_or_else(|| AppError::Validation("Already downloading".into()))?;
    if accept_license {
        state.db.accept_license(&key, &model_id)?;
    }
    let r = imagegen::install(&state.http, &state.image_paths, &hw, &m, cancel, |e| {
        let _ = on_event.send(e);
    })
    .await;
    state.installs.finish(&job);
    r
}

#[tauri::command]
pub fn cancel_image_install(state: State<'_, AppState>, model_id: String) -> bool {
    state.installs.cancel(&format!("img-install:{model_id}"))
}

#[tauri::command]
pub fn delete_image_model(state: State<'_, AppState>, model_id: String) -> AppResult<()> {
    if state.image_gen.active().iter().any(|k| k == &model_id) {
        return Err(AppError::Validation("Wait for the current image to finish first".into()));
    }
    let m = imagegen::catalog().find(&model_id)?;
    imagegen::delete_model(&state.image_paths, m)
}

#[tauri::command]
pub async fn generate_image(
    state: State<'_, AppState>,
    request: GenRequest,
    on_event: Channel<GenEvent>,
) -> AppResult<GeneratedImage> {
    let m = imagegen::catalog().find(&request.model_id)?.clone();
    let paths = state.image_paths.clone();
    if !paths.model_installed(&m) {
        return Err(AppError::NotFound(format!("{} is not installed yet", m.name)));
    }
    let (_, exe) = paths.engine_exe().ok_or_else(|| AppError::NotFound("The image engine is not installed".into()))?;
    let plan = imagegen::plan(&m, &request)?;
    if imagegen::assess(&m, &detect_hw().await?, None).tier == Tier::TooLarge {
        return Err(AppError::Validation(format!("{} needs more memory than this computer has", m.name)));
    }
    // One image at a time: generation uses all available compute.
    let cancel = state.image_gen.start("current").ok_or_else(|| AppError::Validation("An image is already being generated".into()))?;

    let file_name = format!("{}-{}.png", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0), plan.seed);
    let out = paths.images_dir().join(&file_name);
    let args = imagegen::build_args(&paths, &m, &plan, &out);
    let started = std::time::Instant::now();
    let r = imagegen::generate(&exe, &args, &out, cancel, |e| {
        let _ = on_event.send(e);
    })
    .await;
    state.image_gen.finish("current");
    r?;
    state.db.add_image(
        &m.id, &plan.prompt, plan.negative.as_deref(), plan.width, plan.height, plan.steps, plan.seed,
        &file_name, started.elapsed().as_millis() as i64,
    )
}

#[tauri::command]
pub fn cancel_image_generation(state: State<'_, AppState>) -> bool {
    state.image_gen.cancel("current")
}

#[tauri::command]
pub fn list_generated_images(state: State<'_, AppState>) -> AppResult<Vec<GeneratedImage>> {
    state.db.list_images()
}

fn image_path(state: &AppState, id: i64) -> AppResult<(GeneratedImage, std::path::PathBuf)> {
    let img = state.db.get_image(id)?.ok_or_else(|| AppError::NotFound("Image not found".into()))?;
    // file_name is generated by us, but never let it escape the images folder.
    if img.file_name.contains(['/', '\\']) || img.file_name.starts_with('.') {
        return Err(AppError::Validation("invalid image file name".into()));
    }
    let p = state.image_paths.images_dir().join(&img.file_name);
    Ok((img, p))
}

/// PNG bytes as base64 (for display; the CSP allows data: images).
#[tauri::command]
pub fn image_data(state: State<'_, AppState>, id: i64) -> AppResult<String> {
    let (_, p) = image_path(&state, id)?;
    let bytes = std::fs::read(&p).map_err(|_| AppError::NotFound("The image file is missing".into()))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// Copy an image to a location the user picked in a Save dialog.
#[tauri::command]
pub fn export_image(state: State<'_, AppState>, id: i64, dest: String) -> AppResult<()> {
    let (_, p) = image_path(&state, id)?;
    let dest = std::path::PathBuf::from(dest);
    if dest.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()) != Some("png".into()) {
        return Err(AppError::Validation("Save as a .png file".into()));
    }
    std::fs::copy(&p, &dest).map_err(|e| AppError::Internal(format!("Could not save: {e}")))?;
    Ok(())
}

#[tauri::command]
pub fn delete_generated_image(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    let (_, p) = image_path(&state, id)?;
    let _ = std::fs::remove_file(p);
    state.db.delete_image(id)
}
