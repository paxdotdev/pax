fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("app.manifest");
        // Only this user-owned executable carries Windows activation metadata;
        // a dependency's host DLL cannot provide an application manifest.
        println!("cargo:rustc-link-arg-bin=native-desktop-smoke=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=native-desktop-smoke=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}
