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

## D8 — No image/video generation yet (2026-11-21)
Ollama's docs (Jan 2026) state image generation models "currently only work on
macOS"; models are 13 GB+ with unstated memory needs. It could not be built or
verified on Windows here, and the project forbids simulated features. The NL
request parser now detects image/video/audio *generation* requests and tells
the user plainly instead of recommending chat models. Revisit when Ollama ships
Windows support, or evaluate stable-diffusion.cpp as a second runtime.

## D9 — Attachments are read in Rust by path (2026-11-21)
Files come from the native file dialog or window drag-and-drop, which both
yield paths. Rust reads them (size limits, extension allow-list, binary
check), extracts text from PDF (`pdf-extract`, panic-contained; release
profile uses `panic = "unwind"` for this) and DOCX (`zip` + XML), and stores
attachments in SQLite (migration v2). Images go to Ollama only if the model
reports the `vision` capability via `/api/show`.

## D10 — Markdown via react-markdown + remark-gfm (2026-11-21)
No raw HTML rendering; links open in the system browser.
