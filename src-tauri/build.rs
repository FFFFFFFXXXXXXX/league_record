fn main() {
    if std::env::var("PROFILE").unwrap() == "debug" {
        copy_libobs_dependencies();
    }

    copy_libobs_recorder_dependencies();

    // tauri_build::build();

    // panic!()
}

fn copy_libobs_dependencies() {
    let manifest_path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = std::path::Path::new(&manifest_path);
    let libobs_path = manifest_path.join("libobs");
    let target_release_path = manifest_path.join("target/release/");
    let target_debug_path = manifest_path.join("target/debug/");

    let glob_result: glob::Paths = glob::glob(&format!("{}/*.dll", libobs_path.to_str().unwrap())).unwrap();
    let mut copy_paths: Vec<std::path::PathBuf> = glob_result.filter_map(|e| e.ok()).collect();
    copy_paths.push(libobs_path.join("data"));
    copy_paths.push(libobs_path.join("obs-plugins"));

    fs_extra::dir::create_all(&target_release_path, false).expect("failed to create ./target/release/");
    fs_extra::copy_items(
        &copy_paths,
        &target_release_path,
        &fs_extra::dir::CopyOptions {
            overwrite: true,
            ..Default::default()
        },
    )
    .expect("failed to copy libobs dependencies to ./target/release");

    fs_extra::dir::create_all(&target_debug_path, false).expect("failed to create ./target/debug/");
    fs_extra::copy_items(
        &copy_paths,
        &target_debug_path,
        &fs_extra::dir::CopyOptions {
            overwrite: true,
            ..Default::default()
        },
    )
    .expect("failed to copy libobs dependencies to ./target/debug/");
}

fn copy_libobs_recorder_dependencies() {
    let manifest_path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = std::path::Path::new(&manifest_path);

    // artifact path
    let extprocess_recorder_path = std::env::var_os("CARGO_BIN_FILE_LIBOBS_RECORDER_extprocess_recorder")
        .expect("extprocess_recorder artifact dependency is missing");
    let extprocess_recorder_path = std::path::Path::new(&extprocess_recorder_path);
    // target path
    let target_path = std::path::Path::new(&manifest_path).join(format!("libobs/extprocess_recorder.exe"));
    // copy
    std::fs::copy(extprocess_recorder_path, target_path).expect("copying extprocess_recorder artifact failed");
}
