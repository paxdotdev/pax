use std::{env, path::PathBuf};
#[path = "src/sdk.rs"]
mod sdk;
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/sdk.rs");
    println!("cargo:rerun-if-changed=native");
    println!("cargo:rerun-if-env-changed=PAX_HOST_FRAMEWORK_DIR");
    if env::var_os("CARGO_FEATURE_NATIVE").is_none()
        || env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos")
    {
        return;
    }
    let path = if let Some(directory) = env::var_os("PAX_HOST_FRAMEWORK_DIR") {
        PathBuf::from(directory).join("PaxHost.framework/PaxHost")
    } else if env::var_os("CARGO_FEATURE_DESIGNTIME").is_some() {
        // A workstation launcher may target web. Build the SDK only when native
        // execution is explicitly requested, or during application packaging.
        return;
    } else {
        sdk::build(
            &PathBuf::from(env::var_os("OUT_DIR").unwrap()),
            &env::var("CARGO_CFG_TARGET_ARCH").unwrap(),
            env::var("DEBUG").as_deref() == Ok("true"),
        )
        .expect("build native SDK")
    };
    println!("cargo:rustc-env=PAX_HOST_SDK={}", path.display());
}
