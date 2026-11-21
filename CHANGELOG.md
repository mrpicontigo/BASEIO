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
