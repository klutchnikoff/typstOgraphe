fn main() {
    // Cargo and the Tauri CLI must declare exactly the same Fluent languages.
    println!("cargo:rerun-if-changed=../scripts/native-locales.mjs");
    println!("cargo:rerun-if-changed=../../crates/i18n/i18n.toml");
    println!("cargo:rerun-if-changed=../../crates/i18n/i18n");
    let status = std::process::Command::new("node")
        .arg("../scripts/native-locales.mjs")
        .status()
        .expect("Node.js est requis : installer les prérequis indiqués dans le README.");
    assert!(
        status.success(),
        "Impossible de générer les langues natives."
    );
    println!("cargo:rerun-if-changed=Info.plist");
    if std::env::var_os("CARGO_FEATURE_CUSTOM_PROTOCOL").is_some() {
        let overlay: serde_json::Value = std::env::var("TAURI_CONFIG")
            .ok()
            .map(|text| serde_json::from_str(&text).expect("Invalid TAURI_CONFIG"))
            .unwrap_or_default();
        let frontend = overlay
            .pointer("/build/frontendDist")
            .and_then(|v| v.as_str())
            .unwrap_or("../dist");
        assert!(
            std::path::Path::new(frontend).join("index.html").is_file(),
            "Frontend absent : lancer npm ci puis npm run build:web depuis la racine."
        );
    }
    tauri_build::build()
}
