# AI Handoff — BASEIO (Local AI Hub)

_Last updated: 2026-11-21 (Day 1 session)_

Read this first. It is the single source of truth for where the project is.
Also read: ARCHITECTURE.md (how it works), DEVELOPMENT.md (commands, gotchas),
docs/DECISIONS.md (why), ROADMAP.md (what's next).

## Current status
Day 1 + most of Day 2 + first Day 3 feature are **written**.
- Frontend: typechecks, builds, unit tests pass.
- Rust backend: **written but NOT YET COMPILED** — the machine had no Rust/MSVC
  toolchain; they were being installed via winget during this session.
  **First job: compile and fix errors** (see "Exact next task").

## Completed features (code exists)
| Area | Rust | UI |
|---|---|---|
| Hardware detection (CPU+SIMD flags, RAM, GPU via DXGI, disks, OS) | `src-tauri/src/hardware/` | `src/pages/SystemPage.tsx` |
| Curated catalogue (10 families / 22 variants, licences) | `src-tauri/catalog/models.json`, `catalog.rs` | `src/pages/DiscoverPage.tsx` |
| Compatibility tiers + recommendations | `recommend.rs` | Discover page |
| Ollama: status, start, list, running, load/unload, delete | `ollama.rs`, `commands.rs` | `src/pages/ModelsPage.tsx`, `components/RuntimeBanner.tsx` |
| Approval-based install (verified registry size, disk check, licence acceptance, streamed progress, cancel, post-install verify) | `commands.rs::prepare_install/install_model`, `ollama.rs::pull/registry_size` | `components/InstallDialog.tsx`, `lib/installs.tsx` |
| SQLite (conversations, messages, licence acceptances) | `db.rs` | — |
| Streaming chat with history + stop | `commands.rs::send_message`, `ollama.rs::chat` | `src/pages/ChatPage.tsx` |
| Natural-language requests (rule-based) | `intent.rs`, `commands.rs::interpret_request` | `components/AskBox.tsx` |

## Important files
- `src-tauri/src/commands.rs` — every Tauri command (the API surface)
- `src-tauri/src/lib.rs` — app setup, state, command registration
- `src/lib/api.ts` + `src/lib/types.ts` — frontend mirror of the API; keep in sync with Rust serde types
- `src-tauri/catalog/models.json` — curated catalogue (sizes approximate by design)

## Commands
```powershell
npm install
npm run tauri dev          # run app
npm test                   # frontend tests
npm run typecheck
cd src-tauri; cargo test   # Rust tests
npm run tauri build        # installers → src-tauri/target/release/bundle/{nsis,msi}
```

## Known bugs / risks
- Rust code not compiled yet: expect API-shape fixes in `sysinfo 0.37`
  (`RefreshKind::nothing`, `Disk::kind`) and `windows 0.61` DXGI calls.
- `install_model` re-checks disk using the catalogue estimate only (the verified
  size isn't passed back from `prepare_install`).
- Chat messages render as plain text (no Markdown).
- Ollama `start()` spawns `ollama serve`; if the user also has the Ollama tray
  app, it may already be running (handled: we check first).

## Incomplete work
- End-to-end test against real Ollama (install a 1B model, chat, delete).
- Offline behaviour pass. Windows installer build. Ollama bundling/auto-install.
- Live Ollama-library / Hugging Face search.

## Exact next task
1. Open a new terminal (so `cargo` is on PATH), `cd src-tauri; cargo test`.
   Fix compile errors without changing behaviour.
2. `npm run tauri dev`; verify My PC shows real hardware.
3. Install Ollama if missing, then install `qwen3:0.6b` via the UI and chat with it.
4. Update this file's status section.

## Test & build status
| Check | Status |
|---|---|
| `npm run typecheck` | ✅ pass |
| `npm test` (3 tests) | ✅ pass |
| `npm run build` (vite) | ✅ pass |
| `cargo test` | ⏳ not run (toolchain installing) |
| `npm run tauri build` | ⏳ not run |
