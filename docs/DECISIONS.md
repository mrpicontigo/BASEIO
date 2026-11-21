# Decision log

## D1 — Ollama as the first runtime (2026-11-21)
Mature Windows support, GPU auto-detection/offload, built-in resumable,
digest-verified downloads, and an HTTP API. Avoids shipping llama.cpp builds
per GPU vendor. Can also pull Hugging Face GGUFs via `hf.co/org/repo:quant`.

## D2 — DXGI for GPU detection (2026-11-21)
WMI `Win32_VideoController.AdapterRAM` is 32-bit and caps at 4 GB. DXGI
`DedicatedVideoMemory` is 64-bit and needs no external process. GPUs with
<1 GiB dedicated memory are treated as integrated.

## D3 — Curated catalogue embedded in the binary (2026-11-21)
Works offline and gives plain-language descriptions, use-cases and licence
summaries. Sizes are marked approximate; the exact size comes from the
registry manifest at install time. Live search (Ollama/HF) is layered on top
later and never silently replaces curated data.

## D4 — Licence acceptance is mandatory and remembered per licence (2026-11-21)
Every install requires accepting the licence (or a previous acceptance of the
same licence). Unknown models get a per-model key `custom:<tag>`.

## D5 — Tauri Channels for streaming (2026-11-21)
Download progress and chat tokens use `tauri::ipc::Channel` (ordered,
per-call) rather than global events.

## D6 — No router / state library (2026-11-21)
Four pages and one shared download context; `useState` + React context is
enough. Revisit if pages grow.

## D7 — rusqlite (bundled) over tauri-plugin-sql (2026-11-21)
Keeps all data access in Rust behind typed commands; no SQL in the webview.
