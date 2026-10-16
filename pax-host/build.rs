use std::{env, path::PathBuf};
#[path = "src/sdk.rs"]
mod sdk;
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/sdk.rs");
    println!("cargo:rerun-if-changed=native");
    println!("cargo:rerun-if-env-changed=PAX_HOST_FRAMEWORK_DIR");
    println!("cargo:rerun-if-env-changed=PAX_HOST_SDK_DIR");
    let target = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if env::var_os("CARGO_FEATURE_NATIVE").is_none()
        || !matches!(target.as_str(), "macos" | "windows" | "linux")
    {
        return;
    }
    let path = if target == "windows" || target == "linux" {
        let name = if target == "windows" {
            "PaxHost.dll"
        } else {
            "libpax_host.so"
        };
        if let Some(directory) = env::var_os("PAX_HOST_SDK_DIR") {
            PathBuf::from(directory).join(name)
        } else {
            sdk::build_desktop(
                &PathBuf::from(env::var_os("OUT_DIR").unwrap()),
                &target,
                &env::var("CARGO_CFG_TARGET_ARCH").unwrap(),
                env::var("DEBUG").as_deref() == Ok("true"),
            )
            .expect("build desktop native SDK")
        }
    } else if let Some(directory) = env::var_os("PAX_HOST_FRAMEWORK_DIR") {
        PathBuf::from(directory).join("PaxHost.framework/PaxHost")
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
