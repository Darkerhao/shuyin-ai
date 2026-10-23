fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "read_probe",
            "save_probe",
            "list_projects",
            "get_project",
            "save_project",
            "delete_project",
            "list_assets",
            "link_asset",
            "unlink_asset",
            "open_asset",
            "workspace_settings",
            "set_workspace_root",
            "open_project_directory",
            "relocate_workspace",
            "create_backup",
            "restore_backup",
        ]),
    ))
    .expect("Tauri build configuration failed");
}
