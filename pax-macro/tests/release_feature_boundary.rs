use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct FixtureDir(PathBuf);

impl FixtureDir {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pax-macro-release-feature-boundary-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("src")).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for FixtureDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn designtime_macro_feature_is_rejected_when_release_environment_disables_it() {
    let fixture = FixtureDir::new();
    let macro_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = macro_dir.parent().unwrap();
    fs::write(
        fixture.path().join("Cargo.toml"),
        format!(
            r#"[package]
name = "release-feature-boundary-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
pax-macro = {{ path = {:?}, features = ["designtime"] }}
"#,
            macro_dir
        ),
    )
    .unwrap();
    fs::write(
        fixture.path().join("src/lib.rs"),
        r#"use pax_macro::pax;

#[pax]
#[main]
#[inlined(<Group />)]
pub struct App {}
"#,
    )
    .unwrap();

    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .arg("check")
        .arg("--quiet")
        .arg("--offline")
        .current_dir(fixture.path())
        .env("CARGO_TARGET_DIR", workspace_dir.join("target"))
        .env("PAX_BUILD_TARGET", "web")
        .env("PAX_BUILD_DESIGNTIME", "0")
        .env("PAX_BUILD_DESIGNER", "0")
        .env("PAX_DIR", fixture.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "fixture unexpectedly compiled");
    assert!(
        stderr.contains("Pax release builds cannot include the `pax-engine/designtime` feature"),
        "expected release feature boundary error, got:\n{stderr}"
    );
}
