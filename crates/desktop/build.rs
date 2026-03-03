fn main() {
    // Rebuild when frontend dist changes
    println!("cargo:rerun-if-changed=../../ui/dist");
    tauri_build::build();
}
