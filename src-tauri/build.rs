fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&["read_probe", "save_probe"])),
    )
    .expect("Tauri build configuration failed");
}
