fn main() {
    // specify libobs dependency folder
    println!("cargo:rustc-link-search=native=./libobs/");

    tauri_build::build()
}
