//! Real hardware detection. Every value here comes from the OS; nothing is
//! guessed. Fields the OS cannot report are `None`, never invented.

#[cfg(windows)]
mod gpu_windows;

use serde::Serialize;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

#[derive(Debug, Clone, Serialize)]
pub struct CpuInfo {
    pub brand: String,
    pub vendor: String,
    pub physical_cores: Option<usize>,
    pub logical_cores: usize,
    pub frequency_mhz: u64,
    pub arch: String,
    /// SIMD extensions relevant to llama.cpp CPU inference speed.
    pub features: CpuFeatures,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CpuFeatures {
    pub avx: bool,
    pub avx2: bool,
    pub avx512f: bool,
    pub fma: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Microsoft,
    Apple,
    Other,
}

impl GpuVendor {
    pub fn from_pci_id(id: u32) -> Self {
        match id {
            0x10DE => GpuVendor::Nvidia,
            0x1002 | 0x1022 => GpuVendor::Amd,
            0x8086 => GpuVendor::Intel,
            0x1414 => GpuVendor::Microsoft,
            0x106B => GpuVendor::Apple,
            _ => GpuVendor::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: GpuVendor,
    pub vendor_id: u32,
    /// Dedicated VRAM as reported by DXGI (64-bit, accurate above 4 GB).
    pub dedicated_vram_bytes: u64,
    /// System memory the GPU may borrow (integrated GPUs live here).
    pub shared_memory_bytes: u64,
    /// Heuristic: < 1 GiB dedicated memory means an integrated/APU GPU.
    pub integrated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub mount_point: String,
    pub name: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    /// "ssd", "hdd" or "unknown".
    pub kind: String,
    pub removable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OsInfo {
    pub name: String,
    pub version: String,
    pub long_version: String,
    pub hostname: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HardwareInfo {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpus: Vec<GpuInfo>,
    pub disks: Vec<DiskInfo>,
    pub os: OsInfo,
    /// Non-fatal problems encountered during detection (e.g. GPU query failed).
    pub warnings: Vec<String>,
}

impl HardwareInfo {
    /// The GPU with the most dedicated VRAM, if it is a discrete GPU.
    pub fn best_discrete_gpu(&self) -> Option<&GpuInfo> {
        self.gpus
            .iter()
            .filter(|g| !g.integrated)
            .max_by_key(|g| g.dedicated_vram_bytes)
    }
}

const GIB: u64 = 1024 * 1024 * 1024;

fn detect_cpu_features() -> CpuFeatures {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        CpuFeatures {
            avx: std::arch::is_x86_feature_detected!("avx"),
            avx2: std::arch::is_x86_feature_detected!("avx2"),
            avx512f: std::arch::is_x86_feature_detected!("avx512f"),
            fma: std::arch::is_x86_feature_detected!("fma"),
        }
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        CpuFeatures::default()
    }
}

pub fn detect() -> HardwareInfo {
    let mut warnings = Vec::new();

    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );
    // CPU frequency/usage need two samples spaced apart to be meaningful.
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_cpu_all();

    let cpus = sys.cpus();
    let (brand, vendor, freq) = cpus
        .first()
        .map(|c| (c.brand().trim().to_string(), c.vendor_id().to_string(), c.frequency()))
        .unwrap_or_default();

    let cpu = CpuInfo {
        brand,
        vendor,
        physical_cores: System::physical_core_count(),
        logical_cores: cpus.len(),
        frequency_mhz: freq,
        arch: std::env::consts::ARCH.to_string(),
        features: detect_cpu_features(),
    };

    let memory = MemoryInfo {
        total_bytes: sys.total_memory(),
        available_bytes: sys.available_memory(),
    };

    let disks = Disks::new_with_refreshed_list()
        .iter()
        .map(|d| DiskInfo {
            mount_point: d.mount_point().to_string_lossy().to_string(),
            name: d.name().to_string_lossy().to_string(),
            total_bytes: d.total_space(),
            available_bytes: d.available_space(),
            kind: match d.kind() {
                sysinfo::DiskKind::SSD => "ssd",
                sysinfo::DiskKind::HDD => "hdd",
                _ => "unknown",
            }
            .to_string(),
            removable: d.is_removable(),
        })
        .collect();

    let gpus = detect_gpus(&mut warnings);

    let os = OsInfo {
        name: System::name().unwrap_or_else(|| "Unknown".into()),
        version: System::os_version().unwrap_or_default(),
        long_version: System::long_os_version().unwrap_or_default(),
        hostname: System::host_name(),
    };

    HardwareInfo { cpu, memory, gpus, disks, os, warnings }
}

#[cfg(windows)]
fn detect_gpus(warnings: &mut Vec<String>) -> Vec<GpuInfo> {
    match gpu_windows::enumerate() {
        Ok(g) => g,
        Err(e) => {
            warnings.push(format!("GPU detection failed: {e}"));
            Vec::new()
        }
    }
}

#[cfg(not(windows))]
fn detect_gpus(warnings: &mut Vec<String>) -> Vec<GpuInfo> {
    warnings.push("GPU detection is only implemented on Windows".into());
    Vec::new()
}

pub(crate) fn is_integrated(dedicated_vram_bytes: u64) -> bool {
    dedicated_vram_bytes < GIB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_mapping() {
        assert_eq!(GpuVendor::from_pci_id(0x10DE), GpuVendor::Nvidia);
        assert_eq!(GpuVendor::from_pci_id(0x1002), GpuVendor::Amd);
        assert_eq!(GpuVendor::from_pci_id(0x8086), GpuVendor::Intel);
        assert_eq!(GpuVendor::from_pci_id(0xFFFF), GpuVendor::Other);
    }

    #[test]
    fn integrated_heuristic() {
        assert!(is_integrated(128 * 1024 * 1024));
        assert!(!is_integrated(6 * GIB));
    }

    /// Runs against the real machine: values must be plausible, not invented.
    #[test]
    fn detect_real_hardware() {
        let hw = detect();
        assert!(hw.cpu.logical_cores > 0);
        assert!(hw.memory.total_bytes > 512 * 1024 * 1024);
        assert!(hw.memory.available_bytes <= hw.memory.total_bytes);
        assert!(!hw.disks.is_empty());
    }
}
