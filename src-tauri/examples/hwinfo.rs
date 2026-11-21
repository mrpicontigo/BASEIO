//! Dev tool: print detected hardware and top recommendations as JSON.
//! `cargo run --example hwinfo`
fn main() {
    let hw = baseio_lib::hardware::detect();
    println!("{}", serde_json::to_string_pretty(&hw).unwrap());
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let dir = std::path::PathBuf::from(home).join(".ollama").join("models");
    let b = baseio_lib::recommend::Budget::from_hardware(&hw, Some(&dir));
    for r in baseio_lib::recommend::recommend(baseio_lib::catalog::curated(), &b, None).iter().take(6) {
        println!("{:<22} {:?}  {:?}", r.variant.tag, r.compatibility.tier, r.compatibility.reasons);
    }
}
