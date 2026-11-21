//! Hardware-aware compatibility scoring. Pure functions only (no I/O) so the
//! rules are unit-testable. Memory figures are *estimates* and are labelled as
//! such in the UI.

use crate::catalog::{Catalog, CatalogModel, ModelVariant};
use crate::hardware::HardwareInfo;
use serde::Serialize;

const GIB: u64 = 1024 * 1024 * 1024;
/// Memory kept free for Windows and other apps.
const OS_RESERVE: u64 = 2 * GIB;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// Will not fit in memory or on disk.
    TooLarge = 0,
    /// Runs, mostly on CPU; expect slow responses.
    Slow = 1,
    /// Runs at a comfortable speed.
    Good = 2,
    /// Fits entirely in GPU memory.
    Great = 3,
}

#[derive(Debug, Clone, Serialize)]
pub struct Compatibility {
    pub tag: String,
    pub tier: Tier,
    pub estimated_memory_bytes: u64,
    /// Fraction of the model that fits in dedicated VRAM (0.0–1.0).
    pub gpu_fraction: f64,
    pub disk_ok: bool,
    pub reasons: Vec<String>,
}

/// Inputs extracted from `HardwareInfo` so the rules don't depend on detection.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    pub total_ram: u64,
    pub available_ram: u64,
    pub vram: u64,
    pub disk_free: Option<u64>,
    pub avx2: bool,
}

impl Budget {
    pub fn from_hardware(hw: &HardwareInfo, models_dir: Option<&std::path::Path>) -> Self {
        Budget {
            total_ram: hw.memory.total_bytes,
            available_ram: hw.memory.available_bytes,
            vram: hw.best_discrete_gpu().map(|g| g.dedicated_vram_bytes).unwrap_or(0),
            disk_free: models_dir.and_then(|d| disk_free_for(hw, d)),
            avx2: hw.cpu.features.avx2,
        }
    }
}

/// Free space on the disk with the longest mount point that prefixes `path`.
pub fn disk_free_for(hw: &HardwareInfo, path: &std::path::Path) -> Option<u64> {
    let p = path.to_string_lossy().to_lowercase();
    hw.disks
        .iter()
        .filter(|d| p.starts_with(&d.mount_point.to_lowercase()))
        .max_by_key(|d| d.mount_point.len())
        .map(|d| d.available_bytes)
}

/// Rough runtime memory: weights plus KV cache / runtime overhead at the
/// default context size.
pub fn estimate_memory(v: &ModelVariant) -> u64 {
    let overhead = if v.params_b <= 4.0 { GIB / 2 } else if v.params_b <= 9.0 { GIB } else { 2 * GIB };
    (v.approx_size_bytes as f64 * 1.1) as u64 + overhead
}

pub fn assess(v: &ModelVariant, b: &Budget) -> Compatibility {
    let need = estimate_memory(v);
    let usable_ram = b.total_ram.saturating_sub(OS_RESERVE);
    let mut reasons = Vec::new();

    let gpu_fraction = if b.vram == 0 {
        0.0
    } else {
        // Leave ~10% of VRAM for the display and driver.
        ((b.vram as f64 * 0.9) / need as f64).min(1.0)
    };

    let disk_ok = b.disk_free.map_or(true, |f| f > v.approx_size_bytes + GIB);
    if !disk_ok {
        reasons.push("Not enough free disk space for the download".into());
    }

    let fits_memory = need <= usable_ram + (b.vram as f64 * 0.9) as u64;

    let mut tier = if !fits_memory {
        reasons.push("Needs more memory than this computer has".into());
        Tier::TooLarge
    } else if gpu_fraction >= 1.0 {
        reasons.push("Fits entirely in GPU memory".into());
        Tier::Great
    } else if gpu_fraction >= 0.5 {
        reasons.push("Runs partly on the GPU and partly on the CPU".into());
        Tier::Good
    } else if v.params_b <= 4.0 && b.avx2 {
        reasons.push("Small enough to run well on the CPU".into());
        Tier::Good
    } else {
        reasons.push("Runs mainly on the CPU, so replies will be slow".into());
        Tier::Slow
    };

    if !disk_ok {
        tier = Tier::TooLarge;
    }
    if tier != Tier::TooLarge && gpu_fraction < 1.0 && need > b.available_ram {
        reasons.push("Close other apps first: free memory is currently low".into());
    }
    if !b.avx2 && gpu_fraction < 1.0 {
        reasons.push("CPU lacks AVX2, so CPU inference will be slower".into());
    }

    Compatibility { tag: v.tag.clone(), tier, estimated_memory_bytes: need, gpu_fraction, disk_ok, reasons }
}

#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    pub model_id: String,
    pub model_name: String,
    pub variant: ModelVariant,
    pub compatibility: Compatibility,
}

/// For each model, pick the most capable variant that still runs well, then
/// rank models. `use_case` filters (e.g. "coding"); `None` = all chat-capable.
pub fn recommend(cat: &Catalog, b: &Budget, use_case: Option<&str>) -> Vec<Recommendation> {
    let mut out: Vec<Recommendation> = cat
        .models
        .iter()
        .filter(|m| match use_case {
            Some(u) => m.use_cases.iter().any(|x| x == u),
            None => !m.use_cases.iter().all(|x| x == "embeddings"),
        })
        .filter_map(|m| best_variant(m, b))
        .collect();
    out.sort_by(|a, b| {
        b.compatibility
            .tier
            .cmp(&a.compatibility.tier)
            .then(b.variant.params_b.partial_cmp(&a.variant.params_b).unwrap_or(std::cmp::Ordering::Equal))
    });
    out
}

fn best_variant(m: &CatalogModel, b: &Budget) -> Option<Recommendation> {
    m.variants
        .iter()
        .map(|v| (v, assess(v, b)))
        .filter(|(_, c)| c.tier >= Tier::Good)
        .max_by(|(va, ca), (vb, cb)| {
            ca.tier.cmp(&cb.tier).then(va.params_b.partial_cmp(&vb.params_b).unwrap_or(std::cmp::Ordering::Equal))
        })
        .map(|(v, c)| Recommendation {
            model_id: m.id.clone(),
            model_name: m.name.clone(),
            variant: v.clone(),
            compatibility: c,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(params: f64, size_gb: f64) -> ModelVariant {
        ModelVariant {
            tag: format!("t:{params}b"),
            params_b: params,
            quantization: "Q4_K_M".into(),
            approx_size_bytes: (size_gb * GIB as f64) as u64,
            context_length: 8192,
            license: None,
        }
    }

    fn budget(ram_gb: u64, vram_gb: u64) -> Budget {
        Budget { total_ram: ram_gb * GIB, available_ram: ram_gb * GIB / 2, vram: vram_gb * GIB, disk_free: Some(100 * GIB), avx2: true }
    }

    #[test]
    fn fits_in_gpu_is_great() {
        assert_eq!(assess(&v(8.0, 4.7), &budget(16, 12)).tier, Tier::Great);
    }

    #[test]
    fn small_model_on_cpu_is_good() {
        assert_eq!(assess(&v(3.0, 2.0), &budget(8, 0)).tier, Tier::Good);
    }

    #[test]
    fn large_model_on_cpu_is_slow() {
        assert_eq!(assess(&v(8.0, 4.7), &budget(16, 0)).tier, Tier::Slow);
    }

    #[test]
    fn huge_model_does_not_fit() {
        assert_eq!(assess(&v(27.0, 17.0), &budget(8, 4)).tier, Tier::TooLarge);
    }

    #[test]
    fn no_disk_space_blocks() {
        let mut b = budget(64, 24);
        b.disk_free = Some(GIB);
        let c = assess(&v(3.0, 2.0), &b);
        assert_eq!(c.tier, Tier::TooLarge);
        assert!(!c.disk_ok);
    }

    #[test]
    fn recommendations_on_real_catalogue_are_sorted() {
        let r = recommend(crate::catalog::curated(), &budget(16, 8), None);
        assert!(!r.is_empty());
        assert!(r.windows(2).all(|w| w[0].compatibility.tier >= w[1].compatibility.tier));
        assert!(r.iter().all(|x| x.model_id != "nomic-embed-text"));
    }

    #[test]
    fn use_case_filter() {
        let r = recommend(crate::catalog::curated(), &budget(16, 8), Some("coding"));
        assert!(r.iter().any(|x| x.model_id == "qwen2.5-coder"));
        assert!(r.iter().all(|x| x.model_id != "gemma3"));
    }
}
