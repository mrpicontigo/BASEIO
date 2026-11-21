// Mirrors of Rust types in src-tauri/src. Keep in sync when changing serde shapes.

export interface CpuFeatures { avx: boolean; avx2: boolean; avx512f: boolean; fma: boolean }
export interface CpuInfo {
  brand: string; vendor: string; physical_cores: number | null; logical_cores: number;
  frequency_mhz: number; arch: string; features: CpuFeatures;
}
export interface MemoryInfo { total_bytes: number; available_bytes: number }
export type GpuVendor = "nvidia" | "amd" | "intel" | "microsoft" | "apple" | "other";
export interface GpuInfo {
  name: string; vendor: GpuVendor; vendor_id: number; dedicated_vram_bytes: number;
  shared_memory_bytes: number; integrated: boolean;
}
export interface DiskInfo {
  mount_point: string; name: string; total_bytes: number; available_bytes: number;
  kind: "ssd" | "hdd" | "unknown"; removable: boolean;
}
export interface OsInfo { name: string; version: string; long_version: string; hostname: string | null }
export interface HardwareInfo {
  cpu: CpuInfo; memory: MemoryInfo; gpus: GpuInfo[]; disks: DiskInfo[]; os: OsInfo; warnings: string[];
}

export interface License { name: string; url: string; commercial_use: "yes" | "conditional" | "no"; gated: boolean }
export interface ModelVariant {
  tag: string; params_b: number; quantization: string; approx_size_bytes: number;
  context_length: number; license?: License;
}
export interface CatalogModel {
  id: string; name: string; publisher: string; description: string; use_cases: string[];
  license: License; homepage: string; variants: ModelVariant[];
}
export interface Catalog { schema_version: number; updated: string; models: CatalogModel[] }

export type Tier = "too_large" | "slow" | "good" | "great";
export interface Compatibility {
  tag: string; tier: Tier; estimated_memory_bytes: number; gpu_fraction: number;
  disk_ok: boolean; reasons: string[];
}
export interface CatalogAssessment {
  models_dir: string | null; disk_free_bytes: number | null; variants: Compatibility[];
}
export interface Recommendation {
  model_id: string; model_name: string; variant: ModelVariant; compatibility: Compatibility;
}

export interface AppError { kind: string; message: string }

// ---- runtime / install / chat
export interface RuntimeStatus {
  installed: boolean; executable: string | null; running: boolean; version: string | null; base_url: string;
}
export interface InstalledModel {
  name: string; size: number; digest: string; modified_at: string;
  details?: { family?: string; parameter_size?: string; quantization_level?: string } | null;
}
export interface RunningModel { name: string; size: number; size_vram: number; expires_at?: string | null }
export type PullEvent =
  | { event: "status"; message: string }
  | { event: "progress"; completed: number; total: number }
  | { event: "verifying" }
  | { event: "done" };
export interface InstallPlan {
  tag: string; model_name: string | null; in_catalog: boolean;
  verified_size_bytes: number | null; approx_size_bytes: number | null; size_error: string | null;
  license: License; license_key: string; license_accepted: boolean;
  compatibility: Compatibility | null; disk_free_bytes: number | null;
  already_installed: boolean; runtime_running: boolean; blockers: string[];
}
export interface Conversation { id: number; title: string; model: string; created_at: number; updated_at: number }
export interface Attachment {
  name: string; kind: "image" | "text"; mime: string; size_bytes: number;
  text?: string | null; data_base64?: string | null; truncated: boolean;
}
export interface Message {
  id: number; conversation_id: number; role: "system" | "user" | "assistant"; content: string; created_at: number;
  attachments: Attachment[];
}
export interface ModelInfo {
  name: string; capabilities: string[]; family: string | null; parameter_size: string | null;
  quantization_level: string | null; context_length: number | null;
}
export type ChatEvent =
  | { event: "thinking" }
  | { event: "token"; content: string }
  | { event: "done"; eval_count: number | null; eval_duration_ns: number | null };

export interface Intent {
  use_case: string | null; preference: "fast" | "best" | "balanced"; commercial_use_required: boolean;
  max_size_bytes: number | null; mentioned_models: string[]; understood: string[];
  special: { capability: string; available: boolean; message: string } | null;
}
export interface RequestInterpretation { intent: Intent; recommendations: Recommendation[] }

// ---- image generation
export interface ImageModelFile { role: string; repo: string; path: string; size: number; sha256: string }
export interface ImageModel {
  id: string; name: string; publisher: string; description: string; quality: "basic" | "high" | string; verified: boolean;
  license: License; homepage: string; files: ImageModelFile[];
  defaults: { width: number; height: number; steps: number; cfg_scale: number };
  sizes: [number, number][];
}
export interface ImageFit { tier: Tier; estimated_memory_bytes: number; reasons: string[] }
export interface ImageModelStatus { model: ImageModel; installed: boolean; download_bytes: number; fit: ImageFit }
export interface ImageOverview {
  engine_installed: boolean; engine_variant: string | null; engine_download_bytes: number;
  engine_license: License; models: ImageModelStatus[];
}
export type ImageInstallEvent =
  | { event: "status"; message: string }
  | { event: "progress"; completed: number; total: number }
  | { event: "done" };
export type GenEvent = { event: "status"; message: string } | { event: "step"; step: number; total: number };
export interface GenRequest {
  model_id: string; prompt: string; negative_prompt?: string | null; width: number; height: number;
  steps?: number | null; seed?: number | null;
}
export interface GeneratedImage {
  id: number; model_id: string; prompt: string; negative_prompt: string | null; width: number; height: number;
  steps: number; seed: number; file_name: string; duration_ms: number; created_at: number;
}
