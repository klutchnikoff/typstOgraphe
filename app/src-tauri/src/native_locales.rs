//! Keep native localization available when running the embedded frontend without an .app.
// Tauri codegen embeds Info.plist for dev (custom-protocol disabled), but assumes
// a bundle supplies it otherwise. Cargo run/build also need the Mach-O section.
// Do not embed twice in dev, or include application metadata in test executables.
#[cfg(all(target_os = "macos", feature = "custom-protocol", not(test)))]
tauri::embed_plist::embed_info_plist!(concat!(env!("CARGO_MANIFEST_DIR"), "/Info.plist"));
