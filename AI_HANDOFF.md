# AI Handoff — BASEIO (Local AI Hub)

_Last updated: 2026-11-21 (end of Day 1 session)_

Read this first. It is the single source of truth for where the project is.
Also read: ARCHITECTURE.md (how it works), DEVELOPMENT.md (commands, gotchas,
dev tools), docs/DECISIONS.md (why), ROADMAP.md (what's next).

## Current status
Working Windows MVP. Day 1 and Day 2 scope are done and verified against real
hardware and a real Ollama 0.40.16 install. Day 3 is partly done: natural-language
requests, approval-based installs, and the Windows installer build all work.
Remaining Day 3 work: recovery/offline polish and an Ollama-bundling decision.

## Completed features (verified)
| Area | Rust | UI | Verified how |
|---|---|---|---|
| Hardware detection (CPU+SIMD, RAM, GPU via DXGI, disks, OS) | `src-tauri/src/hardware/` | `src/pages/SystemPage.tsx` | `cargo run --example hwinfo` on i7-12700H / 16 GB / Iris Xe — correct |
| Curated catalogue (10 families / 22 variants, licences) | `src-tauri/catalog/models.json`, `catalog.rs` | `DiscoverPage.tsx` | unit tests |
| Compatibility tiers + recommendations | `recommend.rs` | Discover page | unit tests + real hardware |
| Ollama: status, start, list, running, load/unload, delete | `ollama.rs`, `commands.rs` | `ModelsPage.tsx`, `RuntimeBanner.tsx` | e2e (delete only via UI path, not e2e) |
| Approval-based install: exact registry size, disk check, licence acceptance, streamed progress, cancel, post-install verify | `commands.rs::prepare_install/install_model`, `ollama.rs::pull/registry_size` | `InstallDialog.tsx`, `lib/installs.tsx` | e2e pulled qwen3:0.6b (522,653,767 B) |
| SQLite (conversations, messages, licence acceptances, migrations) | `db.rs` | — | unit tests; DB created by app at startup |
| Streaming chat with history, stop, "Thinking…" for reasoning models | `commands.rs::send_message`, `ollama.rs::chat` | `ChatPage.tsx` | e2e chat with qwen3:0.6b |
| Natural-language requests (rule-based, no side effects) | `intent.rs`, `commands.rs::interpret_request` | `components/AskBox.tsx` | unit tests |
| Windows installers | `tauri.conf.json` bundle targets nsis+msi | — | `npm run tauri build` OK |

## Important files
- `src-tauri/src/commands.rs` — every Tauri command (the API surface)
- `src-tauri/src/lib.rs` — app setup, state, command registration
- `src/lib/api.ts` + `src/lib/types.ts` — frontend mirror of the API; keep in sync with Rust serde types
- `src-tauri/catalog/models.json` — curated catalogue (sizes approximate by design; exact size fetched at install)
- `src-tauri/examples/{hwinfo,e2e}.rs` — real-hardware / real-Ollama checks

## Commands
```powershell
npm install
npm run tauri dev                    # run app
npm test; npm run typecheck          # frontend checks
cd src-tauri; cargo test             # Rust tests (28)
cargo run --example e2e              # real Ollama end-to-end (needs Ollama)
npm run tauri build                  # → src-tauri/target/release/bundle/{nsis,msi}
```

## Known bugs / limitations
- `install_model` re-checks disk space using the catalogue estimate, not the
  verified size from `prepare_install` (fine in practice, minor).
- Chat renders plain text (no Markdown / code highlighting).
- Integrated GPUs (Intel Iris Xe etc.) are treated as CPU-only for
  recommendations; Ollama may still use them via Vulkan, so ratings may be pessimistic.
- The installer does not bundle or install Ollama; the app shows a "Get Ollama" link.
- Model delete has not been exercised end-to-end (code path mirrors the tested ones).
- Live search of the Ollama library / Hugging Face is not implemented; only the curated catalogue is browsable.
- Bundle identifier changed to `com.baseio.desktop` (data dir `%APPDATA%\com.baseio.desktop`).

## Incomplete work (priority order)
1. Recovery polish: on startup, detect pulls interrupted by an app close and
   offer resume; surface Ollama crash mid-chat with a "Restart engine" action.
2. Offline pass: with the network disabled, confirm that Discover, My models and
   Chat all work, and that `prepare_install` shows "estimate" instead of failing
   (expected to work by design, but not yet tested).
3. Ollama onboarding: optional consented silent install of OllamaSetup.exe
   (verify hash) from the RuntimeBanner.
4. Markdown rendering in chat.
5. Hugging Face GGUF search → install via `hf.co/org/repo:quant` (gated check via HF API).

## Exact next task
Do item 1 of "Incomplete work". In `lib/installs.tsx`, on mount, call
`api.activeInstalls()`. In Rust, persist in-progress pulls to SQLite (new
migration v2: `pending_installs(tag, started_at)`). On startup, list any pending
installs on the Models page with a Resume button: `install_model` with
`accept_license=false` works because licence acceptance is stored, and Ollama
resumes partial blobs.

## Test & build status (2026-11-21)
| Check | Status |
|---|---|
| `npm run typecheck` | ✅ pass |
| `npm test` (3 tests) | ✅ pass |
| `cargo test` (28 tests, 0 warnings) | ✅ pass |
| `cargo run --example e2e` (real Ollama 0.40.16) | ✅ pass |
| `npm run tauri dev` | ✅ starts, DB created |
| `npm run tauri build` | ✅ NSIS 2.9 MB + MSI 3.9 MB |
