fn main() {
    // The CLI generates this from Fluent before Cargo runs. Development codegen
    // embeds this exact path, whereas the release bundle merges it at packaging.
    println!("cargo:rerun-if-changed=Info.plist");
    tauri_build::build()
}
