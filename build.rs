fn main() {
    // Embed the app icon (and version resources) into the Windows executable.
    // The .ico is generated from assets/icons/starlight.svg.
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=assets/icons/starlight.ico");
        winresource::WindowsResource::new()
            .set_icon("assets/icons/starlight.ico")
            .set("ProductName", "Starlight")
            .set("FileDescription", "Starlight")
            .compile()
            .expect("embed windows icon resource");
    }
}
