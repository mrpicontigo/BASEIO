# Roadmap

## Day 1 — Foundation
- [x] Tauri 2 + React + TS + Tailwind scaffold
- [x] Real hardware detection (CPU, RAM, GPU via DXGI, disks, OS)
- [x] Catalogue architecture (embedded curated JSON, validation)
- [x] Compatibility engine + recommendations
- [x] Desktop UI shell + navigation; Discover and My PC pages
- [x] Docs + unit tests

## Day 2 — Core
- [x] Ollama client: status, start, list, running, load/unload, delete
- [x] Approval-based install: verified registry size, disk check, licence acceptance
- [x] Streaming download progress, cancel, post-install verification
- [x] SQLite persistence; local chat with streaming + stop
- [x] End-to-end test against real Ollama 0.40.16 (`cargo run --example e2e`)
- [ ] Live search of the Ollama library / Hugging Face GGUF models
- [x] Chat attachments: images (vision models), PDF, DOCX, text/code
- [x] Model capability detection (`/api/show`) + text-only warnings
- [x] ChatGPT/Claude-style chat UI with markdown

## Day 3 — Intelligence & release
- [ ] Natural-language model requests ("I want something for coding on my laptop")
      → parsed into use-case + constraints → recommendations → approval dialog
      (rule-based first; optional local-LLM parsing with a strict JSON schema;
      never executes generated commands)
- [ ] Error/recovery polish (Ollama crash mid-download, disk full, offline)
- [ ] Offline test pass (catalogue, installed models, chat must work offline)
- [ ] Windows installer (`npm run tauri build` → NSIS + MSI)
- [ ] Bundling/auto-install of Ollama (silent installer with consent)

## Later
- Hugging Face gated-model handling with user token
- Model detail page, prompt presets, system prompts per chat
- Syntax highlighting in code blocks
- Image generation (blocked: Ollama image models are macOS-only; see D8)
- GPU free-VRAM via NVML/ADLX; tokens/sec benchmark per model
- Additional runtimes (llama.cpp server, LM Studio)
