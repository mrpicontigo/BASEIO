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

## D8 — No image/video generation via Ollama (2026-11-21) — superseded by D11 for images
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

## D11 — Image generation via stable-diffusion.cpp (2026-11-21)
Second runtime alongside Ollama, used only for images. Pinned release
`master-1106-532d2c1` (Windows Vulkan build for any GPU, CPU build otherwise),
downloaded from GitHub and verified against the release's published sha256.
After install, `sd-cli --list-devices` confirms a Vulkan device; if none, the
CPU build is installed instead. Models are multi-file entries in
`catalog/image_models.json`, each file with exact size + sha256 from Hugging
Face LFS metadata; only ungated repos are used. Generation runs `sd-cli.exe`
with argv built from validated values (prompt is a single argument, size from
an allow-list, extra args from a 1-item allow-list). Measured on i7-12700H +
Iris Xe (Vulkan): SD 1.5 Q4_0 512×512, 20 steps ≈ 240 s; 10 steps ≈ 127 s.
Video (`vid_gen` with Wan/LTX) is supported by the engine but not exposed:
needs 12 GB+ VRAM and could not be tested here.
