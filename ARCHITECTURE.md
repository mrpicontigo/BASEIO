# Architecture

```
┌──────────────────────── React + TS (src/) ────────────────────────┐
│ App.tsx shell · pages/ (Discover, Models, Chat, System)            │
│ lib/api.ts  ── typed invoke() wrappers + Channel streams           │
│ lib/installs.tsx ── app-level download manager (React context)     │
└───────────────────────────────┬───────────────────────────────────┘
                                │ Tauri IPC (commands + Channels)
┌───────────────────────────────▼──── Rust (src-tauri/src/) ────────┐
│ commands.rs   thin command layer: validate → delegate → AppError   │
│ hardware/     sysinfo (CPU/RAM/disk/OS) + DXGI (GPU VRAM, Windows) │
│ catalog.rs    embedded curated catalogue + model-ref validation    │
│ recommend.rs  pure compatibility scoring (tiers) + ranking         │
│ ollama.rs     HTTP client for Ollama: status/start/list/pull/chat  │
│ db.rs         SQLite (rusqlite, bundled): chats, licences, settings│
│ state.rs      AppState + cancellable job registry                  │
└───────────────┬─────────────────────────────┬─────────────────────┘
                │ HTTP 127.0.0.1:11434         │ HTTPS
         ┌──────▼──────┐               ┌───────▼────────────────┐
         │ Ollama      │               │ registry.ollama.ai     │
         │ (runtime)   │               │ (manifest = exact size)│
         └─────────────┘               └────────────────────────┘
```

## Key flows
**Recommendation**: `hardware::detect()` → `Budget::from_hardware` (RAM, best
discrete-GPU VRAM, free disk where Ollama stores models, AVX2) →
`recommend::assess(variant, budget)` → tier `great|good|slow|too_large` +
human-readable reasons. Memory need = size × 1.1 + KV/runtime overhead
(0.5/1/2 GiB by size class). Estimates are labelled as such in the UI.

**Install (approval-based)**:
1. `prepare_install(tag)` — no side effects. Fetches the exact size from the
   registry manifest, checks disk, runtime, already-installed, licence status.
   Returns `blockers[]`.
2. UI shows the plan; user must tick licence acceptance (remembered per
   licence in SQLite).
3. `install_model(tag, accept_license, Channel<PullEvent>)` re-validates, then
   streams `/api/pull`. Ollama verifies sha256 per layer; afterwards we confirm
   the model appears in `/api/tags` before reporting success.
4. `cancel_install(tag)` via `watch` channel; Ollama keeps partial blobs so a
   retry resumes.

**Chat**: `send_message(conversation_id, content, Channel<ChatEvent>)` stores
the user message, streams `/api/chat` with full history, stores the reply
(partial reply kept on Stop).

## Security rules
- The only process ever spawned is the located `ollama.exe` with the fixed arg
  `serve`. No shell, no user/AI text on any command line.
- Every model reference passes `catalog::is_valid_model_ref` (strict charset,
  ≤3 path segments) and travels in JSON bodies only.
- CSP restricts the webview to `self` + IPC.

## Data locations
- SQLite: `%APPDATA%\com.baseio.desktop\baseio.db`
- Models: Ollama's store (`OLLAMA_MODELS` or `%USERPROFILE%\.ollama\models`)
