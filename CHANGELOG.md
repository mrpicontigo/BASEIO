# Changelog

## [Unreleased]
### Added
- Create page: local image generation with stable-diffusion.cpp (Vulkan GPU or CPU), verified engine and model downloads, live step progress, stop, gallery, save as PNG. Models: Stable Diffusion 1.5 (tested), Z-Image Turbo (not yet tested).
- Composer focus outline fix.
- Project scaffold (Tauri 2, React 19, TypeScript, Tailwind 4, Vitest).
- Hardware detection: CPU (brand, cores, clock, AVX/AVX2/AVX-512/FMA), RAM,
  GPUs via DXGI (64-bit VRAM), disks (SSD/HDD, free space), OS.
- Curated model catalogue (10 families, 22 variants) with licence info.
- Compatibility tiers and hardware-aware recommendations by use case.
- Ollama integration: detect/start runtime, list/running models,
  load/unload/delete, approval-based install with verified size, licence
  acceptance, streamed progress, cancel, post-install verification.
- SQLite storage; streaming local chat with history and stop.
- Natural-language model requests ("a fast coding model for my laptop").
- "Thinking…" indicator for reasoning models in chat.
- Windows installers (NSIS + MSI) via `npm run tauri build`.
- Dev tools: `cargo run --example hwinfo`, `cargo run --example e2e`.
- Chat attachments: images (vision models only), PDF, Word (.docx), text and code files; drag & drop or paperclip.
- Model capabilities from Ollama (text / sees images / thinks first) in the model picker; warnings for text-only models.
- Markdown replies with code blocks and copy buttons; blank-line gaps collapsed.
- Error screen with Reload instead of a blank window.
### Fixed
- Requests like "generate images" or "video generation" no longer recommend chat models; they explain it is not available.
- Chat header showed "New chat" after the conversation was auto-titled.
### Changed
- Chat redesigned in the style of ChatGPT/Claude: centred conversation, large composer, model picker, grouped history.
- New UI theme: modern minimal, cyan + yellow brand, logo mark, translucent panels.
- Optional animated background sketches (rocket, goat, boat, mountains, dog, books) with an on/off switch in the sidebar (remembered per user).
- Bundle identifier `com.baseio.app` → `com.baseio.desktop` (macOS conflict warning).
