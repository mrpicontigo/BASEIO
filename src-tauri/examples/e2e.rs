//! Dev tool: end-to-end check of the real Ollama integration.
//! `cargo run --example e2e -- [model]` (default qwen3:0.6b, ~520 MB download).
//! Starts Ollama if needed, verifies registry size, pulls, confirms install, chats.
use baseio_lib::ollama::{self, ChatEvent, ChatMessage, Ollama, PullEvent};
use std::io::Write;

fn main() {
    let model = std::env::args().nth(1).unwrap_or_else(|| "qwen3:0.6b".into());
    tauri::async_runtime::block_on(async move {
        let o = Ollama::new();
        println!("status: {:?}", o.status().await);
        let st = o.start().await.expect("start ollama");
        println!("running: {} version {:?}", st.running, st.version);

        let size = ollama::registry_size(&reqwest::Client::new(), &model).await;
        println!("registry size of {model}: {size:?}");

        let (_tx, rx) = tokio::sync::watch::channel(false);
        let mut last = 0u64;
        o.pull(&model, rx, |e| match e {
            PullEvent::Progress { completed, total } => {
                let pct = completed * 100 / total.max(1);
                if pct >= last + 10 { last = pct; println!("  {pct}% of {total}"); }
            }
            other => println!("  {other:?}"),
        })
        .await
        .expect("pull");
        assert!(o.is_installed(&model).await.unwrap());
        println!("installed: {:?}", o.list().await.unwrap().iter().map(|m| &m.name).collect::<Vec<_>>());

        let (_tx, rx) = tokio::sync::watch::channel(false);
        let msgs = [ChatMessage { role: "user".into(), content: "Reply with exactly: hello from baseio /no_think".into(), images: vec![] }];
        print!("chat: ");
        let reply = o
            .chat(&model, &msgs, None, rx, |e| match e {
                ChatEvent::Token { content } => { print!("{content}"); let _ = std::io::stdout().flush(); }
                ChatEvent::Done { eval_count, .. } => println!("\n  [done, {eval_count:?} tokens]"),
                ChatEvent::Thinking => {}
            })
            .await
            .expect("chat");
        assert!(!reply.is_empty());
        println!("running models: {:?}", o.running().await.unwrap().iter().map(|m| (&m.name, m.size_vram)).collect::<Vec<_>>());
        println!("E2E OK");
    });
}
