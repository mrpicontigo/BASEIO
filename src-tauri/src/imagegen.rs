//! Local image generation with stable-diffusion.cpp (`sd-cli.exe`).
//!
//! Safety: the engine is a pinned GitHub release verified by sha256. It is
//! only ever started with arguments built here from validated values: model
//! file paths from the embedded catalogue, a size from the model's allow-list,
//! bounded numbers, and the prompt passed as a single argv entry (no shell).

use crate::catalog::License;
use crate::download;
use crate::error::{AppError, AppResult};
use crate::hardware::HardwareInfo;
use crate::recommend::Tier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::io::AsyncReadExt;
use tokio::sync::watch;

const CATALOG_JSON: &str = include_str!("../catalog/image_models.json");
const GIB: u64 = 1 << 30;
const ALLOWED_EXTRA_ARGS: &[&str] = &["--offload-to-cpu"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Build {
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Engine {
    pub name: String,
    pub tag: String,
    pub license: License,
    pub builds: HashMap<String, Build>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFile {
    pub role: String,
    pub repo: String,
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub cfg_scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageModel {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub description: String,
    pub quality: String,
    /// Generation has been run successfully with this exact configuration.
    #[serde(default)]
    pub verified: bool,
    pub license: License,
    pub homepage: String,
    pub files: Vec<ModelFile>,
    pub defaults: Defaults,
    pub sizes: Vec<[u32; 2]>,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

impl ImageModel {
    pub fn total_size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageCatalog {
    pub schema_version: u32,
    pub engine: Engine,
    pub models: Vec<ImageModel>,
}

fn role_flag(role: &str) -> Option<&'static str> {
    Some(match role {
        "model" => "-m",
        "diffusion_model" => "--diffusion-model",
        "vae" => "--vae",
        "llm" => "--llm",
        "clip_l" => "--clip_l",
        "clip_g" => "--clip_g",
        "t5xxl" => "--t5xxl",
        _ => return None,
    })
}

impl ImageCatalog {
    pub fn parse(json: &str) -> AppResult<Self> {
        let c: ImageCatalog =
            serde_json::from_str(json).map_err(|e| AppError::Catalog(format!("invalid image catalogue: {e}")))?;
        for m in &c.models {
            if m.id.is_empty() || !m.id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
                return Err(AppError::Catalog(format!("bad image model id {}", m.id)));
            }
            if m.files.is_empty() || m.sizes.is_empty() {
                return Err(AppError::Catalog(format!("{}: no files or sizes", m.id)));
            }
            for f in &m.files {
                if role_flag(&f.role).is_none() || f.sha256.len() != 64 || file_name(&f.path).is_none() {
                    return Err(AppError::Catalog(format!("{}: bad file entry {}", m.id, f.path)));
                }
            }
            if m.extra_args.iter().any(|a| !ALLOWED_EXTRA_ARGS.contains(&a.as_str())) {
                return Err(AppError::Catalog(format!("{}: extra arg not allowed", m.id)));
            }
        }
        Ok(c)
    }

    pub fn find(&self, id: &str) -> AppResult<&ImageModel> {
        self.models.iter().find(|m| m.id == id).ok_or_else(|| AppError::NotFound(format!("Unknown image model '{id}'")))
    }
}

pub fn catalog() -> &'static ImageCatalog {
    static C: OnceLock<ImageCatalog> = OnceLock::new();
    C.get_or_init(|| ImageCatalog::parse(CATALOG_JSON).expect("embedded image catalogue is valid"))
}

fn file_name(path: &str) -> Option<&str> {
    path.rsplit('/').next().filter(|n| !n.is_empty() && !n.starts_with('.') && !n.contains('\\'))
}

// ------------------------------------------------------------------ paths

#[derive(Clone)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn engine_dir(&self, variant: &str) -> PathBuf {
        self.root.join("engines").join(format!("sdcpp-{}-{variant}", catalog().engine.tag))
    }
    pub fn model_dir(&self, id: &str) -> PathBuf {
        self.root.join("image-models").join(id)
    }
    pub fn model_file(&self, m: &ImageModel, f: &ModelFile) -> PathBuf {
        self.model_dir(&m.id).join(file_name(&f.path).unwrap_or("model"))
    }
    pub fn images_dir(&self) -> PathBuf {
        self.root.join("images")
    }

    /// Installed engine executable, preferring the GPU (Vulkan) build.
    pub fn engine_exe(&self) -> Option<(String, PathBuf)> {
        ["vulkan", "cpu"].iter().find_map(|v| {
            let p = self.engine_dir(v).join("sd-cli.exe");
            p.is_file().then(|| (v.to_string(), p))
        })
    }

    pub fn model_installed(&self, m: &ImageModel) -> bool {
        m.files.iter().all(|f| download::present(&self.model_file(m, f), f.size))
    }
}

// ------------------------------------------------------------------ fit

#[derive(Debug, Clone, Serialize)]
pub struct ImageFit {
    pub tier: Tier,
    pub estimated_memory_bytes: u64,
    pub reasons: Vec<String>,
}

pub fn assess(m: &ImageModel, hw: &HardwareInfo, disk_free: Option<u64>) -> ImageFit {
    let need = (m.total_size() as f64 * 1.2) as u64 + GIB;
    let vram = hw.best_discrete_gpu().map(|g| g.dedicated_vram_bytes).unwrap_or(0);
    let ram = hw.memory.total_bytes.saturating_sub(2 * GIB);
    let mut reasons = Vec::new();
    let mut tier = if vram as f64 * 0.9 >= need as f64 {
        reasons.push("Fits in GPU memory: images in seconds".into());
        Tier::Great
    } else if vram >= 4 * GIB && ram >= need {
        reasons.push("Runs on your GPU with help from system memory".into());
        Tier::Good
    } else if ram >= need {
        reasons.push(if hw.gpus.is_empty() {
            "Runs on the CPU: expect minutes per image".to_string()
        } else {
            "Runs on integrated graphics / CPU: expect minutes per image".to_string()
        });
        Tier::Slow
    } else {
        reasons.push("Needs more memory than this computer has".into());
        Tier::TooLarge
    };
    if let Some(free) = disk_free {
        if free < m.total_size() + GIB {
            reasons.push("Not enough free disk space".into());
            tier = Tier::TooLarge;
        }
    }
    ImageFit { tier, estimated_memory_bytes: need, reasons }
}

/// Which engine build to install for this machine.
pub fn preferred_variant(hw: &HardwareInfo) -> &'static str {
    if hw.gpus.is_empty() { "cpu" } else { "vulkan" }
}

// ------------------------------------------------------------------ install

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum InstallEvent {
    Status { message: String },
    Progress { completed: u64, total: u64 },
    Done,
}

fn hide_window(cmd: &mut tokio::process::Command) {
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    #[cfg(not(windows))]
    let _ = cmd;
}

/// Does this engine build see a Vulkan GPU? (`sd-cli --list-devices`)
async fn has_vulkan_device(exe: &Path) -> bool {
    let mut cmd = tokio::process::Command::new(exe);
    cmd.arg("--list-devices").kill_on_drop(true);
    hide_window(&mut cmd);
    match tokio::time::timeout(std::time::Duration::from_secs(30), cmd.output()).await {
        Ok(Ok(out)) => String::from_utf8_lossy(&out.stdout).lines().any(|l| l.starts_with("Vulkan")),
        _ => false,
    }
}

async fn install_engine_variant(
    http: &reqwest::Client,
    paths: &Paths,
    variant: &str,
    cancel: &mut watch::Receiver<bool>,
    on_event: &mut impl FnMut(InstallEvent),
    offset: u64,
    grand_total: u64,
) -> AppResult<PathBuf> {
    let dir = paths.engine_dir(variant);
    let exe = dir.join("sd-cli.exe");
    if exe.is_file() {
        return Ok(exe);
    }
    let build = catalog().engine.builds.get(variant).ok_or_else(|| AppError::Internal(format!("no {variant} build")))?;
    on_event(InstallEvent::Status { message: format!("Downloading image engine ({variant})…") });
    let zip = paths.root.join("engines").join(format!("sdcpp-{}-{variant}.zip", catalog().engine.tag));
    download::fetch_verified(http, &build.url, &zip, build.size, &build.sha256, cancel, |c, _| {
        on_event(InstallEvent::Progress { completed: offset + c, total: grand_total })
    })
    .await?;
    on_event(InstallEvent::Status { message: "Unpacking image engine…".into() });
    let tmp = paths.root.join("engines").join(format!(".tmp-{variant}"));
    let _ = std::fs::remove_dir_all(&tmp);
    let (z, t) = (zip.clone(), tmp.clone());
    tauri::async_runtime::spawn_blocking(move || download::extract_zip(&z, &t))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;
    if !tmp.join("sd-cli.exe").is_file() {
        return Err(AppError::Internal("engine archive does not contain sd-cli.exe".into()));
    }
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&tmp, &dir).map_err(|e| AppError::Internal(format!("install engine: {e}")))?;
    let _ = std::fs::remove_file(&zip);
    Ok(exe)
}

/// Install the engine (if needed) and every file of `model`, verifying each.
pub async fn install(
    http: &reqwest::Client,
    paths: &Paths,
    hw: &HardwareInfo,
    model: &ImageModel,
    mut cancel: watch::Receiver<bool>,
    mut on_event: impl FnMut(InstallEvent),
) -> AppResult<()> {
    let need_engine = paths.engine_exe().is_none();
    let variant = preferred_variant(hw);
    let engine_size = if need_engine { catalog().engine.builds[variant].size } else { 0 };
    let missing: Vec<&ModelFile> =
        model.files.iter().filter(|f| !download::present(&paths.model_file(model, f), f.size)).collect();
    let total = engine_size + missing.iter().map(|f| f.size).sum::<u64>();
    let mut done = 0u64;

    if need_engine {
        let exe = install_engine_variant(http, paths, variant, &mut cancel, &mut on_event, 0, total).await?;
        done += engine_size;
        if variant == "vulkan" && !has_vulkan_device(&exe).await {
            on_event(InstallEvent::Status { message: "No usable Vulkan GPU found; installing the CPU engine instead…".into() });
            let _ = std::fs::remove_dir_all(paths.engine_dir("vulkan"));
            install_engine_variant(http, paths, "cpu", &mut cancel, &mut on_event, done, total).await?;
        }
    }

    for f in missing {
        let url = format!("https://huggingface.co/{}/resolve/main/{}", f.repo, f.path);
        on_event(InstallEvent::Status { message: format!("Downloading {}…", file_name(&f.path).unwrap_or("")) });
        let base = done;
        download::fetch_verified(http, &url, &paths.model_file(model, f), f.size, &f.sha256, &mut cancel, |c, _| {
            on_event(InstallEvent::Progress { completed: base + c, total })
        })
        .await?;
        done += f.size;
    }
    on_event(InstallEvent::Done);
    Ok(())
}

pub fn delete_model(paths: &Paths, model: &ImageModel) -> AppResult<()> {
    let dir = paths.model_dir(&model.id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| AppError::Internal(format!("delete {}: {e}", dir.display())))?;
    }
    Ok(())
}

// ------------------------------------------------------------------ generate

#[derive(Debug, Clone, Deserialize)]
pub struct GenRequest {
    pub model_id: String,
    pub prompt: String,
    #[serde(default)]
    pub negative_prompt: Option<String>,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub steps: Option<u32>,
    #[serde(default)]
    pub seed: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum GenEvent {
    Status { message: String },
    Step { step: u32, total: u32 },
}

fn clean_text(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control() || *c == '\n').take(max).collect::<String>().trim().to_string()
}

/// Validated, ready-to-run generation parameters.
#[derive(Debug, Clone)]
pub struct GenPlan {
    pub prompt: String,
    pub negative: Option<String>,
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub seed: i64,
}

pub fn plan(m: &ImageModel, r: &GenRequest) -> AppResult<GenPlan> {
    let prompt = clean_text(&r.prompt, 1500);
    if prompt.is_empty() {
        return Err(AppError::Validation("Describe the image you want".into()));
    }
    if !m.sizes.iter().any(|s| s[0] == r.width && s[1] == r.height) {
        return Err(AppError::Validation(format!("{}x{} is not a supported size for {}", r.width, r.height, m.name)));
    }
    let steps = r.steps.unwrap_or(m.defaults.steps);
    if !(1..=50).contains(&steps) {
        return Err(AppError::Validation("Steps must be between 1 and 50".into()));
    }
    let negative = r.negative_prompt.as_deref().map(|n| clean_text(n, 800)).filter(|n| !n.is_empty());
    let seed = r.seed.filter(|s| *s >= 0).unwrap_or_else(|| {
        (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) % 2_147_483_647) as i64
    });
    Ok(GenPlan { prompt, negative, width: r.width, height: r.height, steps, seed })
}

/// Build argv for sd-cli (everything is a separate argument; no shell).
pub fn build_args(paths: &Paths, m: &ImageModel, p: &GenPlan, out: &Path) -> Vec<String> {
    let mut a: Vec<String> = vec!["-M".into(), "img_gen".into()];
    for f in &m.files {
        a.push(role_flag(&f.role).expect("validated").into());
        a.push(paths.model_file(m, f).to_string_lossy().into_owned());
    }
    a.extend(["-p".into(), p.prompt.clone()]);
    if let Some(n) = &p.negative {
        a.extend(["-n".into(), n.clone()]);
    }
    a.extend([
        "-W".into(), p.width.to_string(),
        "-H".into(), p.height.to_string(),
        "--steps".into(), p.steps.to_string(),
        "--cfg-scale".into(), format!("{}", m.defaults.cfg_scale),
        "-s".into(), p.seed.to_string(),
        "-o".into(), out.to_string_lossy().into_owned(),
    ]);
    a.extend(m.extra_args.iter().cloned());
    a
}

/// Tracks what sd-cli is doing from its log output.
#[derive(Default)]
pub struct LogParser {
    sampling: bool,
    tail: Vec<String>,
}

impl LogParser {
    /// Feed one log line; returns an event worth showing, if any.
    pub fn line(&mut self, line: &str) -> Option<GenEvent> {
        let l = line.trim();
        if l.is_empty() {
            return None;
        }
        if self.tail.len() >= 30 {
            self.tail.remove(0);
        }
        self.tail.push(l.to_string());
        let low = l.to_lowercase();
        // Log lines observed from sd-cli (master-1106):
        //   [I] loading model from '...'            -> loading
        //   [I] generating image: 1/1 - seed 1      -> sampling starts
        //   |=====>   | 1/2 -  12.01s/it            -> a sampling step
        //   |#####    | 308/686 - 1.35GB/s          -> tensor loading (ignored)
        //   [I] sampling completed, taking 22.04s   -> sampling done
        //   [I] decoding 1 latents                  -> VAE decode
        if low.contains("loading model from") {
            return Some(GenEvent::Status { message: "Loading model…".into() });
        }
        if low.contains("generating image:") {
            self.sampling = true;
            return Some(GenEvent::Status { message: "Generating…".into() });
        }
        if low.contains("sampling completed") || low.contains("decoding") {
            self.sampling = false;
            return Some(GenEvent::Status { message: "Finishing image…".into() });
        }
        if self.sampling && (low.contains("s/it") || low.contains("it/s")) {
            if let Some((s, t)) = parse_step(l) {
                return Some(GenEvent::Step { step: s, total: t });
            }
        }
        None
    }

    pub fn error_summary(&self) -> String {
        let errs: Vec<&String> = self.tail.iter().filter(|l| l.to_lowercase().contains("error") || l.contains("failed")).collect();
        let pick = if errs.is_empty() { self.tail.iter().rev().take(3).collect::<Vec<_>>() } else { errs.into_iter().rev().take(3).collect() };
        pick.into_iter().rev().cloned().collect::<Vec<_>>().join(" | ")
    }
}

/// Find "N/M" in a progress line like `|=====>   | 5/20 - 1.23s/it`.
pub fn parse_step(l: &str) -> Option<(u32, u32)> {
    let bar = l.rfind('|')?;
    let rest = &l[bar + 1..];
    let tok = rest.split_whitespace().next()?;
    let (a, b) = tok.split_once('/')?;
    let (s, t) = (a.parse::<u32>().ok()?, b.parse::<u32>().ok()?);
    (t > 0 && s <= t).then_some((s, t))
}

/// Run sd-cli and stream progress. Returns once the PNG exists.
pub async fn generate(
    exe: &Path,
    args: &[String],
    out: &Path,
    mut cancel: watch::Receiver<bool>,
    mut on_event: impl FnMut(GenEvent),
) -> AppResult<()> {
    if let Some(d) = out.parent() {
        std::fs::create_dir_all(d).map_err(|e| AppError::Internal(e.to_string()))?;
    }
    let mut cmd = tokio::process::Command::new(exe);
    cmd.args(args)
        .current_dir(exe.parent().unwrap_or(Path::new(".")))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    hide_window(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| AppError::Runtime(format!("Could not start the image engine: {e}")))?;
    let mut stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");

    let mut parser = LogParser::default();
    let (mut b1, mut b2) = (vec![0u8; 4096], vec![0u8; 4096]);
    let (mut pending, mut out_open, mut err_open) = (String::new(), true, true);
    on_event(GenEvent::Status { message: "Starting image engine…".into() });

    while out_open || err_open {
        let chunk: Vec<u8> = tokio::select! {
            _ = cancel.changed() => {
                let _ = child.kill().await;
                return Err(AppError::Cancelled);
            }
            r = stdout.read(&mut b1), if out_open => match r { Ok(0) | Err(_) => { out_open = false; continue } Ok(n) => b1[..n].to_vec() },
            r = stderr.read(&mut b2), if err_open => match r { Ok(0) | Err(_) => { err_open = false; continue } Ok(n) => b2[..n].to_vec() },
        };
        pending.push_str(&String::from_utf8_lossy(&chunk));
        // Progress bars redraw with '\r', normal logs end with '\n'.
        while let Some(i) = pending.find(['\r', '\n']) {
            let line: String = pending.drain(..=i).collect();
            if let Some(e) = parser.line(&line) {
                on_event(e);
            }
        }
    }
    let status = child.wait().await.map_err(|e| AppError::Runtime(e.to_string()))?;
    if !status.success() || !out.is_file() {
        let detail = parser.error_summary();
        let low = detail.to_lowercase();
        let hint = if low.contains("memory") || low.contains("alloc") {
            " The model may need more memory than is free. Close other apps or pick a smaller size."
        } else {
            ""
        };
        return Err(AppError::Runtime(format!("Image generation failed.{hint} Details: {detail}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalogue_is_valid() {
        let c = catalog();
        assert!(c.engine.builds.contains_key("vulkan") && c.engine.builds.contains_key("cpu"));
        assert!(c.find("sd15-q4").is_ok());
    }

    #[test]
    fn rejects_unsafe_catalogue() {
        let bad = CATALOG_JSON.replace("\"--offload-to-cpu\"", "\"--rpc-servers\"");
        assert!(ImageCatalog::parse(&bad).is_err());
    }

    #[test]
    fn plan_validates() {
        let m = catalog().find("sd15-q4").unwrap();
        let ok = GenRequest { model_id: m.id.clone(), prompt: "a cat\u{0}".into(), negative_prompt: None, width: 512, height: 512, steps: None, seed: Some(42) };
        let p = plan(m, &ok).unwrap();
        assert_eq!(p.prompt, "a cat");
        assert_eq!(p.steps, 20);
        assert!(plan(m, &GenRequest { width: 999, ..ok.clone() }).is_err());
        assert!(plan(m, &GenRequest { prompt: "   ".into(), ..ok.clone() }).is_err());
        assert!(plan(m, &GenRequest { steps: Some(500), ..ok }).is_err());
    }

    #[test]
    fn prompt_is_a_single_argument() {
        let paths = Paths { root: PathBuf::from("C:/x") };
        let m = catalog().find("sd15-q4").unwrap();
        let p = GenPlan { prompt: "cat\" --rpc-servers evil".into(), negative: None, width: 512, height: 512, steps: 20, seed: 1 };
        let a = build_args(&paths, m, &p, Path::new("C:/x/out.png"));
        let i = a.iter().position(|x| x == "-p").unwrap();
        assert_eq!(a[i + 1], "cat\" --rpc-servers evil");
        assert!(!a.iter().any(|x| x == "--rpc-servers"));
    }

    #[test]
    fn progress_parsing() {
        assert_eq!(parse_step("  |=========>         | 5/20 - 1.23s/it"), Some((5, 20)));
        assert_eq!(parse_step("[INFO ] loading tensors"), None);
        // Real sd-cli output (master-1106) on this project's dev machine.
        let mut p = LogParser::default();
        assert!(matches!(p.line("[I] loading model from 'C:/m.gguf' --- diffusion_engine.cpp:690"), Some(GenEvent::Status { .. })));
        assert!(p.line("[I] loading tensors completed, taking 0.02s --- model_loader.cpp:1442").is_none());
        assert!(p.line("[I] sampling using Euler A method --- request.cpp:478").is_none());
        assert!(matches!(p.line("[I] generating image: 1/1 - seed 1 --- image.cpp:1005"), Some(GenEvent::Status { .. })));
        assert!(p.line("  |#######################                           | 308/686 -   1.35GB/s").is_none());
        assert!(matches!(p.line("  |=========================>                        | 1/2 -  12.01s/it - ETA 00:12"), Some(GenEvent::Step { step: 1, total: 2 })));
        assert!(matches!(p.line("[I] sampling completed, taking 22.04s --- image.cpp:1038"), Some(GenEvent::Status { .. })));
        assert!(p.line("  |##########| 76/76 - 1.2GB/s").is_none());
    }
}
