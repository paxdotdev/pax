use pax_compiler::dev_session::{self, DevSession};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Fixture {
    root: tempfile::TempDir,
    project: PathBuf,
    registry: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("worktree-a/project");
        fs::create_dir_all(&project).unwrap();
        let registry = if cfg!(target_os = "macos") {
            root.path()
                .join("Library/Application Support/Pax/dev/sessions")
        } else if cfg!(target_os = "windows") {
            root.path().join("local/Pax/dev/sessions")
        } else {
            root.path().join("state/pax/dev/sessions")
        };
        fs::create_dir_all(&registry).unwrap();
        Self {
            root,
            project,
            registry,
        }
    }

    fn other_project(&self) -> PathBuf {
        let path = self.root.path().join("worktree-b/project");
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn session(&self, id: &str, project: &Path) -> DevSession {
        DevSession {
            session_id: id.to_string(),
            platform: "web".to_string(),
            designtime: true,
            project_root: Some(project.to_path_buf()),
            session_dir: Some(project.join(".pax/dev/sessions").join(id)),
            app_pid: None,
            design_server_addr: None,
            control_kind: "test".to_string(),
            location: None,
            started_at_ms: dev_session::now_ms(),
            last_seen_ms: dev_session::now_ms(),
        }
    }

    fn register(&self, session: &DevSession) {
        fs::write(
            self.registry.join(format!("{}.json", session.session_id)),
            serde_json::to_vec(session).unwrap(),
        )
        .unwrap();
    }

    fn activate(&self, project: &Path, session: &DevSession) {
        dev_session::write_project_active_session(&project.join(".pax"), session).unwrap();
    }

    fn run(&self, args: &[&str]) -> Output {
        // Isolate every platform's registry in the child process, including stale cleanup.
        // Never inspect or send requests to the developer's actual running sessions.
        Command::new(env!("CARGO_BIN_EXE_pax-cli"))
            .arg("dev")
            .args(args)
            .current_dir(&self.project)
            .env("HOME", self.root.path())
            .env("USERPROFILE", self.root.path())
            .env("LOCALAPPDATA", self.root.path().join("local"))
            .env("APPDATA", self.root.path().join("roaming"))
            .env("XDG_STATE_HOME", self.root.path().join("state"))
            .env("PAX_TELEMETRY", "off")
            .env("PAX_API_BASE_URL", "http://127.0.0.1:1")
            .env("CI", "true")
            .output()
            .unwrap()
    }
}

#[track_caller]
fn selected(output: Output) -> DevSession {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[track_caller]
fn failure(output: Output, expected: &str) -> String {
    assert!(
        !output.status.success(),
        "unexpectedly selected: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains(expected), "expected {expected:?} in {error}");
    error
}

#[test]
fn explicit_project_never_falls_back_to_unrelated_session() {
    let fixture = Fixture::new();
    fixture.register(&fixture.session("other-worktree", &fixture.other_project()));
    for flag in ["--path", "-p"] {
        let error = failure(fixture.run(&["status", flag, "."]), "no live dev session");
        assert!(error.contains(fixture.project.canonicalize().unwrap().to_str().unwrap()));
        assert!(error.contains("--path"));
        assert!(error.contains("dev list"));
    }
}

#[test]
fn explicit_project_without_sessions_fails_for_long_and_short_path_flags() {
    let fixture = Fixture::new();
    for flag in ["--path", "-p"] {
        failure(fixture.run(&["status", flag, "."]), "no live dev session");
    }
}

#[test]
fn active_project_session_is_selected_among_multiple_worktrees_and_sessions() {
    let fixture = Fixture::new();
    let first = fixture.session("first-tab", &fixture.project);
    let active = fixture.session("active-tab", &fixture.project);
    for session in [
        &first,
        &active,
        &fixture.session("other", &fixture.other_project()),
    ] {
        fixture.register(session);
    }
    fixture.activate(&fixture.project, &active);
    for args in [vec!["status", "--path", "."], vec!["status"]] {
        assert_eq!(selected(fixture.run(&args)).session_id, "active-tab");
    }
    assert_eq!(
        selected(fixture.run(&["status", "--path", ".", "--session", "first-tab"])).session_id,
        "first-tab"
    );
    failure(
        fixture.run(&["status", "--path", ".", "--session", "missing"]),
        "no live dev session with id missing",
    );
}

#[test]
fn stale_and_unregistered_active_sessions_cannot_trigger_global_fallback() {
    let fixture = Fixture::new();
    let mut stale = fixture.session("stale", &fixture.project);
    stale.last_seen_ms = 0;
    fixture.register(&stale);
    fixture.activate(&fixture.project, &stale);
    fixture.register(&fixture.session("other", &fixture.other_project()));
    failure(
        fixture.run(&["status", "--path", "."]),
        "no live dev session",
    );
    assert!(!fixture.registry.join("stale.json").exists());
    failure(
        fixture.run(&["status", "--session", "stale"]),
        "no live dev session with id stale",
    );
    failure(
        fixture.run(&["status", "--path", "."]),
        "no live dev session",
    );
}

#[test]
fn active_pointer_must_belong_to_the_selected_project() {
    let fixture = Fixture::new();
    let mut session = fixture.session("copied-pointer", &fixture.other_project());
    fixture.register(&session);
    fixture.activate(&fixture.project, &session);
    failure(
        fixture.run(&["status", "--path", "."]),
        "does not belong to project",
    );
    session.project_root = None;
    fixture.register(&session);
    failure(
        fixture.run(&["status", "--path", "."]),
        "does not belong to project",
    );
}

#[test]
fn exact_session_selection_and_explicit_project_constraints_are_both_honored() {
    let fixture = Fixture::new();
    let mut other = fixture.session("other", &fixture.other_project());
    fixture.register(&other);
    assert_eq!(
        selected(fixture.run(&["status", "--session", "other"])).session_id,
        "other"
    );
    failure(
        fixture.run(&["status", "--path", ".", "--session", "other"]),
        "does not belong to project",
    );
    assert_eq!(
        selected(fixture.run(&[
            "status",
            "--path",
            other.project_root.as_ref().unwrap().to_str().unwrap(),
            "--session",
            "other",
        ]))
        .session_id,
        "other"
    );
    failure(
        fixture.run(&["status", "--session", "missing"]),
        "no live dev session with id missing",
    );
    other.project_root = None;
    fixture.register(&other);
    assert_eq!(
        selected(fixture.run(&["status", "--session", "other"])).session_id,
        "other"
    );
    failure(
        fixture.run(&["status", "--path", ".", "--session", "other"]),
        "does not belong to project",
    );
}

#[test]
fn implicit_discovery_retains_only_unambiguous_global_fallback() {
    let fixture = Fixture::new();
    failure(fixture.run(&["status"]), "no live dev sessions found");
    fixture.register(&fixture.session("one", &fixture.other_project()));
    assert_eq!(selected(fixture.run(&["status"])).session_id, "one");
    fixture.register(&fixture.session("two", &fixture.other_project()));
    failure(fixture.run(&["status"]), "multiple live dev sessions found");
    failure(
        fixture.run(&["status", "--path", "."]),
        "no live dev session",
    );
}

#[test]
fn relative_and_absolute_paths_are_normalized() {
    let fixture = Fixture::new();
    let session = fixture.session("matching", &fixture.project);
    fixture.register(&session);
    fixture.activate(&fixture.project, &session);
    fs::create_dir(fixture.project.join("child")).unwrap();
    for path in [".", "child/..", fixture.project.to_str().unwrap()] {
        assert_eq!(
            selected(fixture.run(&["status", "--path", path])).session_id,
            "matching"
        );
    }
}

#[cfg(unix)]
#[test]
fn symlink_aliases_match_in_both_requested_and_registered_paths() {
    let fixture = Fixture::new();
    let alias = fixture.root.path().join("alias");
    std::os::unix::fs::symlink(&fixture.project, &alias).unwrap();
    let mut session = fixture.session("matching", &fixture.project);
    fixture.register(&session);
    fixture.activate(&fixture.project, &session);
    assert_eq!(
        selected(fixture.run(&["status", "--path", alias.to_str().unwrap()])).session_id,
        "matching"
    );
    session.project_root = Some(alias);
    fixture.register(&session);
    assert_eq!(
        selected(fixture.run(&["status", "--path", "."])).session_id,
        "matching"
    );
}

#[test]
fn invalid_project_paths_fail_closed() {
    let fixture = Fixture::new();
    fixture.register(&fixture.session("other", &fixture.other_project()));
    failure(
        fixture.run(&["status", "--path", "missing"]),
        "could not resolve project path",
    );
    fs::write(fixture.project.join("file"), "not a directory").unwrap();
    failure(
        fixture.run(&["status", "--path", "file"]),
        "not a directory",
    );
}

#[test]
fn every_session_command_rejects_an_unmatched_project_before_side_effects() {
    let fixture = Fixture::new();
    let other_project = fixture.other_project();
    fixture.register(&fixture.session("other", &other_project));
    let commands = [
        vec!["status"],
        vec!["look", "--output-dir", "captures", "--timeout-ms", "1"],
        vec!["logs", "--timeout-ms", "1"],
        vec!["ray-cast", "--x", "0", "--y", "0", "--timeout-ms", "1"],
        vec!["selector", "Text", "--timeout-ms", "1"],
        vec!["inspect", "tree", "--timeout-ms", "1"],
        vec![
            "touch",
            "apply-component-source",
            "--component",
            "Example",
            "--source",
            "",
        ],
        vec![
            "touch",
            "replace-node",
            "--component",
            "crate::Example",
            "--template-node-id",
            "1",
            "--source",
            "",
            "--timeout-ms",
            "1",
        ],
    ];
    for mut command in commands {
        command.extend(["--path", "."]);
        failure(fixture.run(&command), "no live dev session");
        command.extend(["--session", "other"]);
        failure(fixture.run(&command), "does not belong to project");
        assert!(
            !other_project.join(".pax").exists(),
            "request sent to unrelated session: {command:?}"
        );
        assert!(!fixture.project.join("captures").exists());
    }
}
