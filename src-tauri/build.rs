fn main() {
    build_helper::build_to_path("./").unwrap();
    copy_libobs_recorder_dependencies();
    tauri_build::build();
}

fn copy_libobs_recorder_dependencies() -> Option<()> {
    use std::{env, fs, path};

    let artifact_path = env::var_os("CARGO_BIN_FILE_LIBOBS_RECORDER_extprocess_recorder")?;
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")?;
    let target_path = path::Path::new(&manifest_dir).join("libobs/extprocess_recorder.exe");

    fs::copy(artifact_path, target_path).ok()?;

    Some(())
}
