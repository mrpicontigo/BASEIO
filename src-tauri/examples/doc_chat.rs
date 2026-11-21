//! Dev tool: real check of document chat. Extracts text from a file with the
//! same code the app uses, then asks the model about it.
//! `cargo run --example doc_chat -- <file> [model]`
use baseio_lib::{attachments, ollama::{ChatEvent, ChatMessage, Ollama}};
use std::io::Write;

fn main() {
    let path = std::env::args().nth(1).expect("usage: doc_chat <file> [model]");
    let model = std::env::args().nth(2).unwrap_or_else(|| "qwen3:0.6b".into());
    let att = attachments::read(std::path::Path::new(&path)).expect("read attachment");
    println!("extracted {} chars (truncated: {})", att.text.as_ref().map_or(0, |t| t.len()), att.truncated);
    let content = format!("{}\nWhat is the secret code word in the file? Answer with just the word. /no_think", attachments::as_prompt_context(&[att]));
    tauri::async_runtime::block_on(async move {
        let o = Ollama::new();
        let (_tx, rx) = tokio::sync::watch::channel(false);
        let msgs = [ChatMessage { role: "user".into(), content, images: vec![] }];
        print!("reply: ");
        let r = o.chat(&model, &msgs, Some(8192), rx, |e| if let ChatEvent::Token { content } = e { print!("{content}"); let _ = std::io::stdout().flush(); }).await;
        println!("\nresult: {}", if r.is_ok() { "OK" } else { "ERROR" });
        r.expect("chat");
    });
}
