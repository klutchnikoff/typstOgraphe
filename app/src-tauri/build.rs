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
        assert!(
            std::path::Path::new("../dist/index.html").is_file(),
            "Frontend absent : lancer npm ci puis npm run build:web depuis la racine."
        );
    }
    tauri_build::build()
}
