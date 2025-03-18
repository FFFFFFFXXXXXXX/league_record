fn main() {
    build_helper::Builder::new().with_path("./target/").build().unwrap();
    tauri_build::build();
}
