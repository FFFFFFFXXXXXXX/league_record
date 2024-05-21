fn main() {
    const LIBOBS_DIR: &str = "./target/";
    build_helper::build_to_path(LIBOBS_DIR).unwrap();
    build_helper::copy_artifact_dependency_to_path(LIBOBS_DIR).unwrap();

    tauri_build::build();
}
