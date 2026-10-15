fn main() {
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(&["get_snapshot", "save_settings", "get_discovery", "refresh_discovery"]));
    if let Err(error) = tauri_build::try_build(attributes) {
        eprintln!("OBTAINER build configuration failed: {error}");
        std::process::exit(1);
    }
}
