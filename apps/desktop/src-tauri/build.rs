fn main() {
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "get_snapshot",
            "save_settings",
            "get_discovery",
            "refresh_discovery",
            "get_manual_routers",
            "add_manual_router",
            "remove_manual_router",
            "probe_management_endpoint",
            "cancel_endpoint_probe",
            "cancel_discovery",
        ]));
    if let Err(error) = tauri_build::try_build(attributes) {
        eprintln!("OBTAINER build configuration failed: {error}");
        std::process::exit(1);
    }
}
