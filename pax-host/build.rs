fn main() {
    println!("cargo:rerun-if-env-changed=PAX_HOST_FRAMEWORK_DIR");
    if let Some(directory) = std::env::var_os("PAX_HOST_FRAMEWORK_DIR") {
        // Cargo also links a package's cdylib while building its binary. Carry
        // the host framework search path to every target using this crate.
        println!(
            "cargo:rustc-link-search=framework={}",
            std::path::Path::new(&directory).display()
        );
    }
}
