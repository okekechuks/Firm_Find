fn main() {
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "select_firmware_file",
            "discard_firmware_selection",
            "start_firmware_import",
            "list_firmware_artifacts",
            "get_firmware_artifact",
            "verify_firmware_integrity",
            "cancel_library_job",
            "list_library_jobs",
            "get_library_job",
            "get_library_activity",
            "export_artifact_manifest",
            "get_profile_registry",
            "get_device_compatibility",
            "validate_device_profile",
            "record_device_observation",
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
