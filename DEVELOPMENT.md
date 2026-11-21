# Development

## Prerequisites (Windows)
| Tool | Install |
|---|---|
| Node.js ≥ 20 | https://nodejs.org |
| Rust (stable, MSVC) | `winget install Rustlang.Rustup` then `rustup default stable-msvc` |
| MSVC C++ Build Tools | `winget install Microsoft.VisualStudio.2022.BuildTools` with the "Desktop development with C++" workload |
| WebView2 | Preinstalled on Windows 11 |
| Ollama (runtime) | `winget install Ollama.Ollama` — required to download/run models |

## Commands
```powershell
npm install              # frontend deps
npm run tauri dev        # run the desktop app with hot reload
npm test                 # frontend unit tests (vitest)
npm run typecheck        # TypeScript check
cd src-tauri; cargo test # Rust unit tests (includes a real hardware-detection test)
npm run tauri build      # release build + NSIS/MSI installers in src-tauri/target/release/bundle
```

## Gotchas
- **Vitest + lowercase drive letter**: Vitest fails with "failed to find the
  current suite" if the cwd is `c:\...` instead of `C:\...` (VS Code terminals
  do this). `npm test` goes through `scripts/vitest.mjs`, which fixes it. Don't
  call `npx vitest` directly from a lowercase path.
- After installing Rust, open a **new** terminal so `cargo` is on PATH.

## Conventions
- Rust types returned to the UI are mirrored in `src/lib/types.ts`. Change both.
- All Tauri commands live in `src-tauri/src/commands.rs`; logic lives in modules.
- Every model reference passed to the runtime must pass `catalog::is_valid_model_ref`.
- Never execute shell commands built from user or AI text.

## Dev tools (run from `src-tauri/`)
- `cargo run --example hwinfo` — prints detected hardware JSON + top recommendations.
- `cargo run --example e2e -- qwen3:0.6b` — real end-to-end check against Ollama:
  start → registry size → pull with progress → verify installed → streamed chat.
  Downloads ~520 MB the first time; re-runs are instant.

## Toolchain notes (this machine)
- Rust installed with official `rustup-init.exe` (stable-msvc). `~/.cargo/bin`
  is on PATH only in new terminals; in old ones prefix `$HOME/.cargo/bin`.
- VS 2022 Build Tools (C++ workload) and Ollama 0.40.16 installed via winget.
