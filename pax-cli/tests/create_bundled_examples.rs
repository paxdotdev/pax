use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn run_create(cli: &Path, destination: &Path, extra: &[&str]) -> std::process::Output {
    let mut command = Command::new(cli);
    command.arg("create").arg(destination).args(extra);
    command.output().expect("failed to run pax-cli create")
}

fn run_web_until_serving(cli: &Path, project: &Path) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut child = Command::new(cli)
        .args(["run", "--target=web", "--hot-reload=off"])
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_TARGET_DIR", workspace.join("target/pax-create-e2e"))
        .current_dir(project)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("failed to run created default project");
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = sender.send(line);
        }
    });

    let deadline = Instant::now() + Duration::from_secs(120);
    let mut served = false;
    let mut output = Vec::new();
    while Instant::now() < deadline {
        if let Ok(line) = receiver.recv_timeout(Duration::from_millis(250)) {
            if line.contains("Server running at http://") {
                served = true;
                break;
            }
            output.push(line);
        }
        if let Some(status) = child.try_wait().unwrap() {
            panic!(
                "created default project exited before serving: {status}\n{}",
                output.join("\n")
            );
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    assert!(
        served,
        "created default project did not start serving web\n{}",
        output.join("\n")
    );
}

#[test]
fn creates_bundled_examples_outside_the_monorepo() {
    let cli = Path::new(env!("CARGO_BIN_EXE_pax-cli"));
    let temp = tempfile::tempdir().unwrap();
    let default = temp.path().join("hello-pax");
    let override_project = temp.path().join("tiny-counter");
    let missing = temp.path().join("not-created");

    assert!(run_create(cli, &default, &[]).status.success());
    assert!(run_create(cli, &override_project, &["--example=increment"])
        .status
        .success());
    let invalid = run_create(cli, &missing, &["--example=missing"]);
    assert!(!invalid.status.success());
    let error = String::from_utf8_lossy(&invalid.stderr);
    assert!(error.contains("living-quilt"));
    assert!(error.contains("ink-and-light"));
    assert!(error.contains("increment"));
    assert!(!missing.exists());

    let default_manifest = fs::read_to_string(default.join("Cargo.toml")).unwrap();
    assert!(default_manifest.contains("name = \"hello-pax\""));
    assert!(default_manifest.contains("title = \"hello-pax\""));
    assert!(default_manifest.contains("pax-kit = { version ="));
    assert!(!default_manifest.contains("../../../pax-kit"));
    assert!(default_manifest.contains("[profile.dev]\nopt-level = 1"));
    assert!(default_manifest.contains("[profile.dev.package.hello-pax]\nopt-level = 0"));
    assert!(!default_manifest.contains("profile.dev.package.living-quilt"));
    assert!(default.join("src/quilt_tile.pax").is_file());
    assert!(default.join("src/animated_pax_logo.pax").is_file());
    assert!(default.join("src/animated_pax_logo_banner.rs").is_file());
    assert!(fs::read_to_string(default.join("pax"))
        .unwrap()
        .contains("pax-cli \"$@\""));

    assert!(fs::read_to_string(override_project.join("src/lib.pax"))
        .unwrap()
        .contains("num_clicks"));
    let override_manifest = fs::read_to_string(override_project.join("Cargo.toml")).unwrap();
    assert!(override_manifest.contains("[profile.dev]\nopt-level = 1"));
    assert!(override_manifest.contains("[profile.dev.package.tiny-counter]\nopt-level = 0"));
    assert!(!override_manifest.contains("profile.dev.package.increment"));
    assert!(!override_project.join(".pax").exists());
    assert!(!override_project.join("target").exists());

    if std::env::var_os("PAX_CREATE_E2E_WEB").is_some() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let cargo_config = default.join(".cargo/config.toml");
        fs::create_dir_all(cargo_config.parent().unwrap()).unwrap();
        fs::write(
            cargo_config,
            format!(
                "[patch.crates-io]\npax-kit = {{ path = {:?} }}\n",
                workspace.join("pax-kit")
            ),
        )
        .unwrap();
        let status = Command::new(cli)
            .args(["build", "--target=web"])
            .env("CARGO_NET_OFFLINE", "true")
            .env("CARGO_TARGET_DIR", workspace.join("target/pax-create-e2e"))
            .current_dir(&default)
            .status()
            .expect("failed to build created default project");
        assert!(status.success());
        run_web_until_serving(cli, &default);
    }
}
