//! Workstation development entry shared by Cargo applications and the compiler.
//! Compilation never calls this module; an application's explicit `run` does.
use crate::{HotReloadMode, RunContext, RunTarget};
use clap::{App, Arg, ErrorKind};
use color_eyre::eyre::{bail, eyre, Result};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// A parsed development request. Preparing it neither builds nor starts the UI.
pub struct Launch(Action);
enum Action {
    Run(RunContext),
    Help(String),
    Worker(Vec<OsString>),
}

fn command() -> App<'static, 'static> {
    App::new("pax")
        .bin_name("cargo run -- pax")
        .template("Pax development\n\n{about}\n\nUSAGE:\n    {usage}\n\n{all-args}")
        .about("Build and launch this Pax application without an installed pax-cli")
        .arg(
            Arg::with_name("target")
                .long("target")
                .takes_value(true)
                .possible_values(&["web", "macos", "ios", "ipados", "ipad"]),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .takes_value(true)
                .help("Web server port; fail if the requested port is occupied"),
        )
        .arg(
            Arg::with_name("release")
                .long("release")
                .help("Build the target application without designtime in release mode"),
        )
        .arg(
            Arg::with_name("build")
                .long("build")
                .help("Package the application without serving, installing or launching"),
        )
        .arg(
            Arg::with_name("hot-reload")
                .long("hot-reload")
                .takes_value(true)
                .possible_values(&["pax", "off", "all", "logic"]),
        )
        .arg(
            Arg::with_name("ios-device")
                .long("ios-device")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("ios-development-team")
                .long("ios-development-team")
                .takes_value(true),
        )
        .arg(Arg::with_name("verbose").long("verbose"))
}

/// Read the reserved leading `pax` namespace, or use project defaults when no
/// arguments are supplied. Custom application parsers should use `init_with!`
/// for explicit native startup instead of implicitly consuming developer flags.
pub fn prepare(project: &Path) -> Result<Launch> {
    prepare_args(
        project,
        std::env::args_os().skip(1).collect(),
        std::env::var("PAX_TARGET").ok().as_deref(),
    )
}

fn prepare_args(
    project: &Path,
    mut args: Vec<OsString>,
    environment_target: Option<&str>,
) -> Result<Launch> {
    if !args.is_empty() {
        if args[0] != "pax" {
            bail!("Pax development options must follow `pax`: cargo run -- pax --target=web. For application-owned arguments and in-process native startup, use init_with!.");
        }
        args.remove(0);
    }
    if args.first().is_some_and(|arg| arg == "__designtime-server") {
        args.remove(0);
        return Ok(Launch(Action::Worker(args)));
    }
    args.insert(0, "pax".into());
    let matches = match command().get_matches_from_safe(args) {
        Ok(matches) => matches,
        Err(error) if error.kind == ErrorKind::HelpDisplayed => {
            return Ok(Launch(Action::Help(error.to_string())))
        }
        Err(error) => return Err(eyre!(error.to_string())),
    };
    let document: toml_edit::Document =
        std::fs::read_to_string(project.join("Cargo.toml"))?.parse()?;
    let configured_target = document
        .get("package")
        .and_then(|v| v.get("metadata"))
        .and_then(|v| v.get("pax"))
        .and_then(|v| v.get("dev"))
        .and_then(|v| v.get("target"))
        .and_then(|v| v.as_str());
    let target = match matches
        .value_of("target")
        .or(environment_target)
        .or(configured_target)
        .unwrap_or("web")
    {
        "web" => RunTarget::Web,
        "macos" => RunTarget::macOS,
        "ios" => RunTarget::iOS,
        "ipados" | "ipad" => RunTarget::iPadOS,
        other => bail!("Unsupported Pax target `{other}`; choose web, macos, ios or ipados"),
    };
    if !cfg!(target_os = "macos") && target != RunTarget::Web {
        bail!("Apple application targets require a macOS workstation with Xcode; use --target=web on this workstation");
    }
    let port = matches
        .value_of("port")
        .map(str::parse::<u16>)
        .transpose()
        .map_err(|_| eyre!("--port must be between 0 and 65535"))?;
    if port.is_some() && target != RunTarget::Web {
        bail!("--port configures the web target only");
    }
    let is_release = matches.is_present("release");
    Ok(Launch(Action::Run(RunContext {
        target,
        port,
        embedded_launcher: true,
        project_path: project.to_path_buf(),
        verbose: matches.is_present("verbose"),
        should_also_run: !matches.is_present("build"),
        is_libdev_mode: is_repository_project(project),
        process_child_ids: Arc::new(Mutex::new(Vec::new())),
        should_run_designtime: !is_release,
        hot_reload: if is_release {
            Some(HotReloadMode::Off)
        } else {
            matches
                .value_of("hot-reload")
                .map(str::parse::<HotReloadMode>)
                .transpose()
                .map_err(|error| eyre!(error))?
        },
        is_release,
        profile_wasm_size: false,
        ios_device: matches.value_of("ios-device").map(str::to_owned),
        ios_development_team: matches.value_of("ios-development-team").map(str::to_owned),
        lifecycle_observer: None,
    })))
}

impl Launch {
    /// Build and run the selected target. Development servers own this call
    /// until stopped. No application initializer executes in the build tools.
    pub fn run(self) -> Result<()> {
        match self.0 {
            Action::Help(help) => {
                print!("{help}");
                Ok(())
            }
            Action::Worker(args) => run_worker(args),
            Action::Run(ctx) => {
                let children = Arc::clone(&ctx.process_child_ids);
                ctrlc::set_handler(move || {
                    cleanup_children(&children);
                    std::process::exit(130);
                }).map_err(|error| eyre!("Development launch requires ownership of Ctrl-C handling: {error}. Use explicit native initialization when embedding Pax."))?;
                let result = crate::perform_build(&ctx).map(|_| ());
                cleanup_children(&ctx.process_child_ids);
                result
            }
        }
    }
}

/// Whether the project belongs to this checkout's example or test corpus.
pub fn is_repository_project(project: &Path) -> bool {
    let Some(root) = Path::new(env!("CARGO_MANIFEST_DIR")).parent() else {
        return false;
    };
    let Ok(root) = root.canonicalize() else {
        return false;
    };
    let Ok(project) = project.canonicalize() else {
        return false;
    };
    root.join("pax-cli/Cargo.toml").is_file()
        && (project.starts_with(root.join("examples")) || project.starts_with(root.join("tests")))
}

fn cleanup_children(children: &Arc<Mutex<Vec<u64>>>) {
    for id in children.lock().unwrap().drain(..) {
        #[cfg(unix)]
        unsafe {
            // Compiler children lead their own process groups. Never signal
            // the invoking application's process group or unrelated sessions.
            if libc::kill(-(id as i32), libc::SIGTERM) != 0 {
                libc::kill(id as i32, libc::SIGTERM);
            }
        }
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/T", "/PID", &id.to_string()])
                .status();
        }
    }
}

fn run_worker(args: Vec<OsString>) -> Result<()> {
    let mut parser = App::new("pax-internal-server");
    for name in [
        "serve-dir",
        "watch-dir",
        "manifest-path",
        "hot-reload",
        "port",
        "ready-file",
        "macos-session-dir",
    ] {
        parser = parser.arg(Arg::with_name(name).long(name).takes_value(true));
    }
    let matches = parser
        .arg(Arg::with_name("suppress-address-log").long("suppress-address-log"))
        .get_matches_from_safe(
            std::iter::once(OsString::from("pax-internal-server")).chain(args),
        )?;
    let required = |name| {
        matches
            .value_of(name)
            .ok_or_else(|| eyre!("Internal server is missing --{name}"))
    };
    let manifest_path = PathBuf::from(required("manifest-path")?);
    let (manifest, restart) =
        crate::design_server::decode_restart_manifest(&std::fs::read(&manifest_path)?)?;
    let logic = matches.value_of("macos-session-dir").map(|directory| {
        crate::design_server::LogicReloadConfig::Native(
            crate::design_server::NativeLogicReloadConfig {
                session_dir: directory.into(),
                manifest_path: manifest_path.clone(),
            },
        )
    });
    crate::design_server::start_server(
        required("serve-dir")?,
        required("watch-dir")?,
        manifest,
        Some(required("port")?.parse()?),
        matches.value_of("ready-file").map(PathBuf::from),
        !matches.is_present("suppress-address-log"),
        None,
        logic,
        required("hot-reload")?
            .parse::<HotReloadMode>()
            .map_err(|error| eyre!(error))?,
        restart,
        Some(manifest_path),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname='app'\nversion='0.1.0'\n[package.metadata.pax.dev]\ntarget='web'\n",
        )
        .unwrap();
        dir
    }
    fn parse(project: &Path, args: &[&str], env: Option<&str>) -> Result<Launch> {
        prepare_args(project, args.iter().map(OsString::from).collect(), env)
    }
    #[test]
    fn namespace_flags_and_precedence() {
        let dir = project();
        let Launch(Action::Run(ctx)) = parse(
            dir.path(),
            &["pax", "--target=web", "--port=8123"],
            Some("invalid"),
        )
        .unwrap() else {
            panic!()
        };
        assert_eq!(ctx.target, RunTarget::Web);
        assert_eq!(ctx.port, Some(8123));
        assert!(ctx.should_run_designtime);
        assert!(parse(dir.path(), &["--target=web"], None).is_err());
        assert!(parse(dir.path(), &["pax", "web"], None).is_err());
        assert!(parse(dir.path(), &["pax", "--port=65536"], None).is_err());
        assert!(parse(dir.path(), &[], Some("invalid")).is_err());
    }
    #[test]
    fn defaults_help_and_production() {
        let dir = project();
        assert!(matches!(
            parse(dir.path(), &[], None).unwrap().0,
            Action::Run(_)
        ));
        assert!(matches!(
            parse(dir.path(), &["pax", "--help"], None).unwrap().0,
            Action::Help(_)
        ));
        let Launch(Action::Run(ctx)) =
            parse(dir.path(), &["pax", "--release", "--build"], None).unwrap()
        else {
            panic!()
        };
        assert!(ctx.is_release);
        assert!(!ctx.should_run_designtime);
        assert!(!ctx.should_also_run);
        assert_eq!(ctx.hot_reload, Some(HotReloadMode::Off));
    }
}
