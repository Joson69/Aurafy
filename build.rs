fn main() {
    println!("cargo:rerun-if-changed=assets/aurafy.ico");
    #[cfg(target_os = "windows")]
    winresource::WindowsResource::new()
        .set_icon("assets/aurafy.ico")
        .set("ProductName", "Aurafy")
        .set("FileDescription", "Aurafy")
        .set("InternalName", "aurafy")
        .set("OriginalFilename", "aurafy.exe")
        .compile()
        .expect("Could not embed the Aurafy Windows icon");
}
