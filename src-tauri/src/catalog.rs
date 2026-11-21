//! Model catalogue. The curated catalogue is compiled into the binary so the
//! app works offline; live sources (Ollama registry, Hugging Face) are layered
//! on top in later modules and never replace the curated data silently.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const CURATED_JSON: &str = include_str!("../catalog/models.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub name: String,
    pub url: String,
    /// "yes" | "conditional" | "no"
    pub commercial_use: String,
    /// True if the publisher requires accepting terms / requesting access.
    pub gated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVariant {
    /// Ollama model reference, e.g. `llama3.2:3b`.
    pub tag: String,
    pub params_b: f64,
    pub quantization: String,
    /// Approximate; verify against the registry manifest before install.
    pub approx_size_bytes: u64,
    pub context_length: u64,
    /// Optional per-variant licence override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogModel {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub description: String,
    pub use_cases: Vec<String>,
    pub license: License,
    pub homepage: String,
    pub variants: Vec<ModelVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub schema_version: u32,
    pub updated: String,
    pub models: Vec<CatalogModel>,
}

impl Catalog {
    pub fn parse(json: &str) -> AppResult<Self> {
        let c: Catalog = serde_json::from_str(json)
            .map_err(|e| AppError::Catalog(format!("invalid catalogue: {e}")))?;
        c.validate()?;
        Ok(c)
    }

    fn validate(&self) -> AppResult<()> {
        let mut seen = std::collections::HashSet::new();
        for m in &self.models {
            if m.variants.is_empty() {
                return Err(AppError::Catalog(format!("model {} has no variants", m.id)));
            }
            for v in &m.variants {
                if !is_valid_model_ref(&v.tag) {
                    return Err(AppError::Catalog(format!("invalid tag {}", v.tag)));
                }
                if !seen.insert(v.tag.clone()) {
                    return Err(AppError::Catalog(format!("duplicate tag {}", v.tag)));
                }
            }
        }
        Ok(())
    }

    pub fn find_variant(&self, tag: &str) -> Option<(&CatalogModel, &ModelVariant)> {
        self.models
            .iter()
            .find_map(|m| m.variants.iter().find(|v| v.tag == tag).map(|v| (m, v)))
    }
}

pub fn curated() -> &'static Catalog {
    static CAT: OnceLock<Catalog> = OnceLock::new();
    CAT.get_or_init(|| Catalog::parse(CURATED_JSON).expect("embedded catalogue is valid"))
}

/// Strict allow-list for model references passed to the runtime.
/// Accepts `name`, `name:tag`, `namespace/name:tag` and `hf.co/org/repo:quant`.
/// This is the single gate every install/run/delete request goes through, so
/// user or AI-generated text can never smuggle extra arguments.
pub fn is_valid_model_ref(s: &str) -> bool {
    if s.is_empty() || s.len() > 200 {
        return false;
    }
    let (path, tag) = match s.rsplit_once(':') {
        Some((p, t)) => (p, Some(t)),
        None => (s, None),
    };
    if let Some(t) = tag {
        if t.is_empty()
            || !t.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        {
            return false;
        }
    }
    let segs: Vec<&str> = path.split('/').collect();
    if segs.len() > 3 {
        return false;
    }
    segs.iter().all(|seg| {
        !seg.is_empty()
            && !seg.starts_with('.')
            && !seg.starts_with('-')
            && seg.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalogue_parses() {
        let c = curated();
        assert!(!c.models.is_empty());
        assert!(c.find_variant("llama3.2:3b").is_some());
    }

    #[test]
    fn model_ref_validation() {
        for ok in ["llama3.2", "llama3.2:3b", "hf.co/bartowski/Llama-3.2-1B-Instruct-GGUF:Q4_K_M", "user/model:latest"] {
            assert!(is_valid_model_ref(ok), "{ok}");
        }
        for bad in ["", "a b", "x;rm -rf", "../etc", "a:", "a:b c", "--help", "a/b/c/d", "m:$(x)", "m\n"] {
            assert!(!is_valid_model_ref(bad), "{bad:?}");
        }
    }

    #[test]
    fn rejects_duplicate_tags() {
        let json = r#"{"schema_version":1,"updated":"x","models":[
          {"id":"a","name":"A","publisher":"p","description":"d","use_cases":[],"homepage":"h",
           "license":{"name":"MIT","url":"u","commercial_use":"yes","gated":false},
           "variants":[{"tag":"a:1","params_b":1,"quantization":"q","approx_size_bytes":1,"context_length":1},
                       {"tag":"a:1","params_b":1,"quantization":"q","approx_size_bytes":1,"context_length":1}]}]}"#;
        assert!(Catalog::parse(json).is_err());
    }
}
