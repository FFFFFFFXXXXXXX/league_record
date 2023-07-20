fn main() {
    copy_libobs_recorder_dependencies();
    tauri_build::build();
}

fn copy_libobs_recorder_dependencies() {
    // artifact path
    if let Some(extprocess_recorder_path) = std::env::var_os("CARGO_BIN_FILE_LIBOBS_RECORDER_extprocess_recorder") {
        let extprocess_recorder_path = std::path::Path::new(&extprocess_recorder_path);

        // target path
        let manifest_path = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
        let manifest_path = std::path::Path::new(&manifest_path);
        let target_path = std::path::Path::new(&manifest_path).join(format!("libobs/extprocess_recorder.exe"));

        // copy
        std::fs::copy(extprocess_recorder_path, target_path).expect("copying extprocess_recorder artifact failed");
    }
}
