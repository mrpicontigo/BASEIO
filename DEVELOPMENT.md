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
