//! Dev tool: real end-to-end image generation using the app's own code.
//! `cargo run --example imagegen_e2e -- [model_id] [prompt]`
//! Installs the engine + model into %APPDATA%\com.baseio.desktop (verified
//! downloads; skips files already present), then generates one image.
use baseio_lib::{hardware, imagegen};

fn main() {
    let id = std::env::args().nth(1).unwrap_or_else(|| "sd15-q4".into());
    let prompt = std::env::args().nth(2).unwrap_or_else(|| "a cute dog sitting next to a stack of books, digital art".into());
    let root = std::path::PathBuf::from(std::env::var("APPDATA").unwrap()).join("com.baseio.desktop");
    let paths = imagegen::Paths { root };
    let m = imagegen::catalog().find(&id).unwrap().clone();
    let hw = hardware::detect();
    println!("fit: {:?}", imagegen::assess(&m, &hw, None));

    tauri::async_runtime::block_on(async move {
        let http = reqwest::Client::new();
        let (_tx, rx) = tokio::sync::watch::channel(false);
        let mut last = 0u64;
        imagegen::install(&http, &paths, &hw, &m, rx, |e| match e {
            imagegen::InstallEvent::Progress { completed, total } => {
                let pct = completed * 100 / total.max(1);
                if pct >= last + 20 { last = pct; println!("  install {pct}%"); }
            }
            other => println!("  {other:?}"),
        })
        .await
        .expect("install");
        let (variant, exe) = paths.engine_exe().expect("engine installed");
        println!("engine: {variant} at {}", exe.display());

        let req = imagegen::GenRequest {
            model_id: id.clone(), prompt, negative_prompt: None,
            width: m.defaults.width, height: m.defaults.height, steps: None, seed: Some(42),
        };
        let plan = imagegen::plan(&m, &req).unwrap();
        let out = paths.images_dir().join(format!("e2e-{id}.png"));
        let _ = std::fs::remove_file(&out);
        let args = imagegen::build_args(&paths, &m, &plan, &out);
        let (_tx, rx) = tokio::sync::watch::channel(false);
        let t = std::time::Instant::now();
        imagegen::generate(&exe, &args, &out, rx, |e| println!("  {e:?}")).await.expect("generate");
        println!("IMAGE OK: {} ({} bytes) in {:.0}s", out.display(), std::fs::metadata(&out).unwrap().len(), t.elapsed().as_secs_f64());
    });
}
