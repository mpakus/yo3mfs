fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=assets/branding/yo3mfs.ico");
    println!("cargo:rerun-if-env-changed=RC_PATH");
    #[cfg(feature = "desktop")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/branding/yo3mfs.ico")
            .set("ProductName", "YO3MFS")
            .set("FileDescription", "YO3MFS — Offline 3MF viewer")
            .compile()?;
    }
    Ok(())
}
