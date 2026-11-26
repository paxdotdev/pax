use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=PAX_PORTAL_AGENT_BINARY");
    println!("cargo:rustc-check-cfg=cfg(bundled_portal_agent)");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let packaged = PathBuf::from("agent")
        .join(env::var("TARGET").unwrap())
        .join("pax-portal-agent");
    let input = env::var_os("PAX_PORTAL_AGENT_BINARY").map(PathBuf::from);
    let path = input.as_ref().unwrap_or(&packaged);
    println!("cargo:rerun-if-changed={}", path.display());
    if input.is_none() && !path.is_file() {
        return;
    }
    let path = fs::canonicalize(path).expect("Locate the packaged Pax Portal Agent");
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(fs::read(&path).expect("Read the packaged Pax Portal Agent"));
    println!("cargo:rustc-cfg=bundled_portal_agent");
    println!("cargo:rustc-env=PAX_BUNDLED_AGENT_PATH={}", path.display());
    println!("cargo:rustc-env=PAX_BUNDLED_AGENT_DIGEST={digest:x}");
}
