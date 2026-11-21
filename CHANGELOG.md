# Changelog

## [Unreleased]
### Added
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
### Changed
- New UI theme: modern minimal, cyan + yellow brand, logo mark, translucent panels.
- Optional animated background sketches (rocket, goat, boat, mountains, dog, books) with an on/off switch in the sidebar (remembered per user).
- Bundle identifier `com.baseio.app` → `com.baseio.desktop` (macOS conflict warning).
