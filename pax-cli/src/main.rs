use clap::{crate_version, App, AppSettings, Arg, ArgMatches};
use color_eyre::config::HookBuilder;
use colored::{ColoredString, Colorize};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use std::{process, thread};

use pax_compiler::{CreateContext, HotReloadMode, RunContext, RunLifecycleObserver, RunTarget};
extern crate pax_language_server;

mod dev;
mod docs;
mod http;
mod svg_import;
mod telemetry;

use color_eyre::eyre::eyre;
use color_eyre::eyre::Report;
use color_eyre::eyre::Result;
use ctrlc;

const UPDATE_COMPLETION_WAIT: Duration = Duration::from_millis(250);

/// `pax-cli` entrypoint
fn main() -> Result<(), Report> {
    HookBuilder::default()
        .display_location_section(false)
        .install()?;

    //Shared state to store child processes keyed by static unique string IDs, for cleanup tracking
    let process_child_ids: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(vec![]));
    // Shared state to store the new version info if available.
    let new_version_info = Arc::new(Mutex::new(None));

    let matches = cli().get_matches_from(normalize_cli_args(std::env::args().collect())?);
    if let ("telemetry", Some(args)) = matches.subcommand() {
        return telemetry::handle(args);
    }
    let public_command = telemetry::public_command(&matches);
    let telemetry = if public_command.is_some() {
        telemetry::TelemetrySession::for_public_command()
    } else {
        telemetry::TelemetrySession::disabled()
    };

    // Updates remain independent of telemetry; internal and management commands
    // do not need to contact the service.
    let update_completion = if public_command.is_some() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let version_info = Arc::clone(&new_version_info);
        thread::spawn(move || {
            http::check_for_update(version_info);
            let _ = sender.try_send(());
        });
        Some(receiver)
    } else {
        None
    };
    let is_libdev_mode = resolve_matches_libdev_mode(&matches)?;

    let cloned_version_info = Arc::clone(&new_version_info);
    let cloned_process_child_ids = Arc::clone(&process_child_ids);
    ctrlc::set_handler(move || {
        println!("\nInterrupt received. Cleaning up child processes...");
        perform_cleanup(
            Arc::clone(&cloned_version_info),
            Arc::clone(&cloned_process_child_ids),
            is_libdev_mode,
            true,
        );
    })
    .expect("ctrl-c hook should have been set up successfully");

    let res = perform_nominal_action(
        matches,
        Arc::clone(&process_child_ids),
        telemetry.lifecycle_observer(),
    );
    let pending_telemetry =
        public_command.and_then(|command| telemetry.command_finished(command, res.is_ok()));
    if let Some(completion) = update_completion {
        let _ = completion.recv_timeout(UPDATE_COMPLETION_WAIT);
    }
    perform_cleanup(new_version_info, process_child_ids, is_libdev_mode, false);
    if let Some(pending) = pending_telemetry {
        pending.wait();
    }
    res
}

fn cli() -> App<'static, 'static> {
    #[allow(non_snake_case)]
    let ARG_PATH = Arg::with_name("path")
        .short("p")
        .long("path")
        .takes_value(true)
        .default_value(".");

    #[allow(non_snake_case)]
    let ARG_VERBOSE = Arg::with_name("verbose")
        .short("v")
        .long("verbose")
        .takes_value(false);

    const DEFAULT_TARGET: &str = "web";
    #[allow(non_snake_case)]
    let ARG_TARGET = Arg::with_name("target")
        .short("t")
        .long("target")
        .default_value(DEFAULT_TARGET)
        .possible_values(&["web", "macos", "ios", "ipados", "ipad"])
        .help("Specify the target platform on which to run. Supported targets include web, macos, ios, and ipados (`ipad` is accepted as an alias).")
        .takes_value(true);

    #[allow(non_snake_case)]
    let ARG_RELEASE = Arg::with_name("release")
        .long("release")
        .takes_value(false)
        .help("Build in Release mode, with appropriate platform-specific optimizations.");

    #[allow(non_snake_case)]
    let ARG_PROFILING = Arg::with_name("profiling")
        .long("profiling")
        .takes_value(false)
        .help("Build an optimized web bundle with Wasm names preserved for size profiling.");

    #[allow(non_snake_case)]
    let ARG_HOT_RELOAD = Arg::with_name("hot-reload")
        .long("hot-reload")
        .takes_value(true)
        .possible_values(&["all", "pax", "logic", "off"])
        .help("Select live source-update lanes: pax (default), all, logic, or off.");

    #[allow(non_snake_case)]
    let ARG_IOS_DEVICE = Arg::with_name("ios-device")
        .long("ios-device")
        .takes_value(true)
        .help("Select the iOS/iPadOS destination. Use `simulator` (default for `run`), `device`, `simulator:<name-or-udid>`, `device:<name-or-udid>`, or an exact device name/UDID.");

    #[allow(non_snake_case)]
    let ARG_IOS_DEVELOPMENT_TEAM = Arg::with_name("ios-development-team")
        .long("ios-development-team")
        .takes_value(true)
        .help("Set the Apple Development Team ID for iOS device builds, e.g. `TWM39MH96F`.");

    #[allow(non_snake_case)]
    let ARG_LIBDEV = Arg::with_name("libdev")
        .long("libdev")
        .takes_value(false)
        .conflicts_with("libdev-mode")
        .help("Signal to the compiler to run certain operations in libdev mode, offering certain ergonomic affordances for Pax library developers.")
        .hidden(true); //hidden because this is of negative value to end-users; things are expected to break when invoked outside of the pax monorepo

    #[allow(non_snake_case)]
    let ARG_LIBDEV_MODE = Arg::with_name("libdev-mode")
        .long("libdev-mode")
        .takes_value(true)
        .possible_values(&["auto", "true", "false"])
        .help("Controls libdev behavior. Auto enables libdev for Pax monorepo examples/tests only.")
        .hidden(true);

    App::new("pax")
        .name("pax")
        .bin_name("pax-cli")
        .about("Pax CLI including compiler and dev tooling")
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .author("Zack Brown <zack@pax.dev>")
        .subcommand(
            App::new("run")
                .about("Run the Pax project from the current working directory in a demo harness")
                .arg( ARG_PATH.clone() )
                .arg( ARG_TARGET.clone() )
                .arg( ARG_IOS_DEVICE.clone() )
                .arg( ARG_IOS_DEVELOPMENT_TEAM.clone() )
                .arg( ARG_VERBOSE.clone() )
                .arg( ARG_LIBDEV.clone() )
                .arg( ARG_LIBDEV_MODE.clone() )
                .arg( ARG_HOT_RELOAD.clone() )
                .arg( ARG_RELEASE.clone().help("Run an optimized local iOS/iPadOS build. Disables designtime and hot reload; does not publish the app.") )
        )
        .subcommand(
            App::new("build")
                .about("Builds the Pax project from the current working directory into a platform-specific executable, for the specific `target` platform.")
                .arg( ARG_PATH.clone() )
                .arg( ARG_TARGET.clone() )
                .arg( ARG_IOS_DEVICE.clone() )
                .arg( ARG_IOS_DEVELOPMENT_TEAM.clone() )
                .arg( ARG_VERBOSE.clone() )
                .arg( ARG_LIBDEV.clone() )
                .arg( ARG_LIBDEV_MODE.clone() )
                .arg( ARG_RELEASE.clone() )
                .arg( ARG_PROFILING.clone() )
        )
        .subcommand(
            App::new("clean")
                .arg( ARG_PATH.clone() )
                .arg( ARG_LIBDEV.clone() )
                .arg( ARG_LIBDEV_MODE.clone() )
                .about("Cleans the temporary files associated with the Pax project in the current working directory — notably, the temporary files generated into the .pax directory")
        )
        .subcommand(
            App::new("create")
                .about("Creates a new Pax + Rust project at the specified path, including necessary boilerplate and default configuration.")
                .alias("new")
                .arg(Arg::with_name("path")
                    .help("File system path where the new project should be created. It should directly follow 'create'")
                    .takes_value(true)
                    .required(true)
                    .index(1))  // Positional arg, `pax create positional_arg_here`
                .arg( ARG_LIBDEV.clone())
                .arg( ARG_LIBDEV_MODE.clone())
                .arg(Arg::with_name("example")
                    .long("example")
                    .takes_value(true)
                    .value_name("name")
                    .help("Create from a curated bundled example (default: living-quilt)."))
        )
        .subcommand(
            App::new("libdev")
                .subcommand(
                    App::new("parse")
                        .arg( ARG_PATH.clone() )
                        .about("Parses the Pax program at the specified path and prints the manifest object, serialized to string. Also prints error messages if parsing fails.")
                )
                .about("Collection of tools for internal library development")
        )
        .subcommand(App::new("lsp").about("Start the Pax LSP server"))
        .subcommand(
            App::new("format")
                .about("Format Pax source files")
                .alias("fmt")
                .arg(Arg::with_name("format-path")
                    .help("File or directory to format (defaults to the current workspace)")
                    .takes_value(true)
                    .default_value(".")
                    .index(1))
                .arg(Arg::with_name("check")
                    .long("check")
                    .help("Check formatting without writing files")
                    .takes_value(false))
        )
        .subcommand(
            App::new("eject")
                .about("Ejects the chassis interface for the target platform")
                .arg( ARG_TARGET.clone())
                .arg( ARG_LIBDEV.clone())
                .arg( ARG_LIBDEV_MODE.clone())
        )
        .subcommand(
            App::new("designtime-server")
                .setting(AppSettings::Hidden)
                .arg(
                    Arg::with_name("serve-dir")
                        .long("serve-dir")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("watch-dir")
                        .long("watch-dir")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("manifest-path")
                        .long("manifest-path")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("macos-session-dir")
                        .long("macos-session-dir")
                        .takes_value(true)
                        .hidden(true),
                )
                .arg(
                    Arg::with_name("port")
                        .long("port")
                        .takes_value(true)
                        .default_value("0"),
                )
                .arg(
                    Arg::with_name("ready-file")
                        .long("ready-file")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("suppress-address-log")
                        .long("suppress-address-log")
                        .takes_value(false)
                        .hidden(true),
                )
                .arg(ARG_HOT_RELOAD.clone().hidden(true)),
        )
        .subcommand(docs::command())
        .subcommand(dev::command())
        .subcommand(svg_import::command())
        .subcommand(telemetry::command())
}

fn run_context(
    args: &ArgMatches<'_>,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
) -> Result<RunContext> {
    let target = parse_run_target(args.value_of("target").unwrap())?;
    let path = PathBuf::from(args.value_of("path").unwrap());
    let is_release = args.is_present("release");
    if is_release && !matches!(target, RunTarget::iOS | RunTarget::iPadOS) {
        return Err(eyre!(
            "run --release currently supports ios and ipados; use build --release for other targets"
        ));
    }
    Ok(RunContext {
        target,
        is_libdev_mode: resolve_libdev_mode(args, &path)?,
        project_path: path,
        verbose: args.is_present("verbose"),
        should_also_run: true,
        process_child_ids,
        should_run_designtime: !is_release,
        // Release cartridges never include source-update machinery, even if
        // --hot-reload or the project/environment requests a live lane.
        hot_reload: if is_release {
            Some(HotReloadMode::Off)
        } else {
            parse_hot_reload_mode(args)?
        },
        is_release,
        profile_wasm_size: false,
        ios_device: args.value_of("ios-device").map(str::to_string),
        ios_development_team: args.value_of("ios-development-team").map(str::to_string),
        lifecycle_observer: None,
    })
}

fn perform_nominal_action(
    matches: ArgMatches<'_>,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
    lifecycle_observer: Option<Arc<dyn RunLifecycleObserver>>,
) -> Result<(), Report> {
    match matches.subcommand() {
        ("run", Some(args)) => {
            let mut context = run_context(args, process_child_ids)?;
            context.lifecycle_observer = lifecycle_observer;
            let _ = pax_compiler::perform_build(&context)?;

            Ok(())
        }
        ("build", Some(args)) => {
            let target = args.value_of("target").unwrap().to_lowercase();
            let path = args.value_of("path").unwrap().to_string(); //default value "."
            let verbose = args.is_present("verbose");
            let is_libdev_mode = resolve_libdev_mode(args, Path::new(&path))?;
            let profile_wasm_size = args.is_present("profiling");
            let is_release = args.is_present("release") || profile_wasm_size;
            let ios_device = args.value_of("ios-device").map(str::to_string);
            let ios_development_team = args.value_of("ios-development-team").map(str::to_string);
            let should_run_designtime = !is_release;
            if profile_wasm_size && target != "web" {
                return Err(eyre!(
                    "--profiling is currently only supported for web builds"
                ));
            }

            let _ = pax_compiler::perform_build(&RunContext {
                target: parse_run_target(&target)?,
                project_path: PathBuf::from(path),
                should_also_run: false,
                should_run_designtime,
                hot_reload: None,
                verbose,
                is_libdev_mode,
                process_child_ids,
                is_release,
                profile_wasm_size,
                ios_device,
                ios_development_team,
                lifecycle_observer: None,
            })?;

            Ok(())
        }
        ("clean", Some(args)) => {
            println!("🧹 Cleaning cached & temporary files...");
            let path = args.value_of("path").unwrap().to_string(); //default value "."

            pax_compiler::perform_clean(&path);

            println!("Done.");
            Ok(())
        }
        ("create", Some(args)) => {
            let path = args.value_of("path").unwrap().to_string(); //default value "."
            let is_libdev_mode = resolve_libdev_mode(args, Path::new(&path))?;
            let version = crate_version!().to_string(); // Note: this could also be parameterized, but an easy default is to clamp to the CLI version

            pax_compiler::perform_create(&CreateContext {
                path,
                is_libdev_mode,
                version,
                example: args.value_of("example").map(str::to_string),
            })
            .map_err(|error| eyre!(error))?;
            Ok(())
        }
        ("eject", Some(args)) => {
            let target = args.value_of("target").unwrap().to_lowercase();
            let is_libdev_mode = resolve_libdev_mode(args, Path::new("."))?;

            let _ = pax_compiler::perform_eject(&RunContext {
                target: parse_run_target(&target)?,
                project_path: PathBuf::from("."),
                should_also_run: false,
                should_run_designtime: false,
                hot_reload: None,
                verbose: false,
                is_libdev_mode,
                process_child_ids,
                is_release: false,
                profile_wasm_size: false,
                ios_device: None,
                ios_development_team: None,
                lifecycle_observer: None,
            })?;

            Ok(())
        }
        ("libdev", Some(args)) => {
            match args.subcommand() {
                ("parse", Some(args)) => {
                    let path = args.value_of("path").unwrap().to_string(); //default value "."
                    let manifest =
                        pax_compiler::static_analysis::build_manifest(&PathBuf::from(path))?;
                    println!("{}", serde_json::to_string_pretty(&vec![manifest])?);

                    Ok(())
                }
                _ => {
                    unreachable!()
                }
            }
        }
        ("lsp", Some(_)) => {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(pax_language_server::start_server());
            Ok(())
        }
        ("format", Some(args)) => {
            let format_path = PathBuf::from(args.value_of("format-path").unwrap());
            let check = args.is_present("check");
            let summary = pax_language::formatting::format_path(&format_path, check)?;
            if check && !summary.changed_files.is_empty() {
                let paths = summary
                    .changed_files
                    .iter()
                    .map(|path| format!("  {}", path.display()))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(eyre!(
                    "{} file(s) require formatting:\n{}",
                    summary.changed_files.len(),
                    paths
                ));
            }

            if check {
                println!("Checked {} Pax source file(s)", summary.files_checked);
            } else {
                println!(
                    "Formatted {} of {} Pax source file(s)",
                    summary.changed_files.len(),
                    summary.files_checked
                );
            }
            Ok(())
        }
        ("designtime-server", Some(args)) => {
            let serve_dir = args.value_of("serve-dir").unwrap();
            let watch_dir = args.value_of("watch-dir").unwrap();
            let manifest_path = PathBuf::from(args.value_of("manifest-path").unwrap());
            let manifest_bytes = std::fs::read(&manifest_path)?;
            let (manifest, restart_snapshot) =
                pax_compiler::design_server::decode_restart_manifest(&manifest_bytes)?;
            let port = args
                .value_of("port")
                .unwrap()
                .parse::<u16>()
                .map_err(|_| eyre!("--port must be an unsigned 16-bit integer"))?;
            let ready_file = args.value_of("ready-file").map(PathBuf::from);
            let show_address_log = !args.is_present("suppress-address-log");
            let hot_reload = parse_hot_reload_mode(args)?.unwrap_or_default();
            let logic_reload = args.value_of("macos-session-dir").map(|session_dir| {
                pax_compiler::design_server::LogicReloadConfig::Native(
                    pax_compiler::design_server::NativeLogicReloadConfig {
                        session_dir: PathBuf::from(session_dir),
                        manifest_path: manifest_path.clone(),
                    },
                )
            });
            pax_compiler::design_server::start_server(
                serve_dir,
                watch_dir,
                manifest,
                Some(port),
                ready_file,
                show_address_log,
                None,
                logic_reload,
                hot_reload,
                restart_snapshot,
                Some(manifest_path),
            )?;
            Ok(())
        }
        ("docs", Some(args)) => docs::handle(args),
        ("dev", Some(args)) => dev::handle(args, process_child_ids),
        ("svg-import", Some(args)) => svg_import::handle(args),
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}

fn parse_hot_reload_mode(args: &ArgMatches<'_>) -> Result<Option<HotReloadMode>, Report> {
    args.value_of("hot-reload")
        .map(str::parse)
        .transpose()
        .map_err(|error: String| eyre!(error))
}

fn parse_run_target(target: &str) -> Result<RunTarget, Report> {
    RunTarget::parse(target).map_err(|error| eyre!(error))
}

fn normalize_cli_args(args: Vec<String>) -> Result<Vec<String>, Report> {
    let mut normalized = Vec::with_capacity(args.len());
    let mut iter = args.into_iter().peekable();

    while let Some(arg) = iter.next() {
        if arg == "--libdev" {
            match iter.peek().map(String::as_str) {
                Some("true") | Some("false") | Some("auto") => {
                    let value = iter.next().unwrap();
                    normalized.push(format!("--libdev-mode={value}"));
                }
                _ => normalized.push(arg),
            }
        } else if let Some(value) = arg.strip_prefix("--libdev=") {
            if value == "true" || value == "false" || value == "auto" {
                normalized.push(format!("--libdev-mode={value}"));
            } else {
                return Err(eyre!(
                    "`--libdev` only accepts `true`, `false`, or `auto`, got `{}`",
                    value
                ));
            }
        } else {
            normalized.push(arg);
        }
    }

    Ok(normalized)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LibdevMode {
    Auto,
    Enabled,
    Disabled,
}

fn resolve_matches_libdev_mode(matches: &ArgMatches<'_>) -> Result<bool, Report> {
    match matches.subcommand() {
        ("run", Some(args)) | ("build", Some(args)) | ("clean", Some(args)) => {
            let path = args.value_of("path").unwrap_or(".");
            resolve_libdev_mode(args, Path::new(path))
        }
        ("create", Some(args)) => {
            let path = args.value_of("path").unwrap_or(".");
            resolve_libdev_mode(args, Path::new(path))
        }
        ("eject", Some(args)) => resolve_libdev_mode(args, Path::new(".")),
        ("libdev", _) => Ok(true),
        _ => Ok(false),
    }
}

fn resolve_libdev_mode(args: &ArgMatches<'_>, project_path: &Path) -> Result<bool, Report> {
    match parse_libdev_mode(args)? {
        LibdevMode::Enabled => Ok(true),
        LibdevMode::Disabled => Ok(false),
        LibdevMode::Auto => Ok(auto_libdev_mode_for_project(project_path)),
    }
}

fn parse_libdev_mode(args: &ArgMatches<'_>) -> Result<LibdevMode, Report> {
    if args.is_present("libdev") {
        return Ok(LibdevMode::Enabled);
    }

    match args.value_of("libdev-mode") {
        Some("true") => Ok(LibdevMode::Enabled),
        Some("false") => Ok(LibdevMode::Disabled),
        Some("auto") | None => Ok(LibdevMode::Auto),
        Some(value) => Err(eyre!(
            "`--libdev` only accepts `true`, `false`, or `auto`, got `{}`",
            value
        )),
    }
}

fn auto_libdev_mode_for_project(project_path: &Path) -> bool {
    let Some(workspace_root) = local_pax_workspace_root() else {
        return false;
    };
    let project_path = absolute_path(project_path);

    [
        workspace_root.join("examples").join("src"),
        workspace_root.join("tests").join("src"),
    ]
    .iter()
    .any(|libdev_root| project_path.starts_with(libdev_root))
}

fn local_pax_workspace_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?;
    let root = root.canonicalize().ok()?;

    let required_members = [
        "pax-cli",
        "pax-compiler",
        "pax-runtime",
        "pax-std",
        "pax-chassis-web",
    ];
    if !root.join("Cargo.toml").is_file()
        || !required_members
            .iter()
            .all(|member| root.join(member).is_dir())
    {
        return None;
    }

    Some(root)
}

fn absolute_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };

    if let Ok(canonical) = absolute.canonicalize() {
        return canonical;
    }

    let Some(parent) = absolute.parent() else {
        return absolute;
    };
    let Some(file_name) = absolute.file_name() else {
        return absolute;
    };
    parent
        .canonicalize()
        .map(|parent| parent.join(file_name))
        .unwrap_or(absolute)
}

fn perform_cleanup(
    new_version_info: Arc<Mutex<Option<String>>>,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
    is_libdev_mode: bool,
    force_end_process: bool,
) {
    //1. kill any running child processes
    if let Ok(process_child_ids_lock) = process_child_ids.lock() {
        process_child_ids_lock.iter().for_each(|child_id| {
            kill_process(*child_id as u32)
                .expect(&format!("Failed to kill process with ID: {}", child_id));
        });
    }

    //2. print update message if appropriate
    if let Ok(new_version_lock) = new_version_info.lock() {
        if !is_libdev_mode {
            if let Some(new_version) = new_version_lock.as_ref() {
                if new_version != "" {
                    //Print our banner if we have a concrete value stored in the new version mutex
                    const TOTAL_LENGTH: usize = 60;
                    let stars_line: ColoredString =
                        "*".repeat(TOTAL_LENGTH).bright_white().on_bright_black();
                    let empty_line: ColoredString =
                        " ".repeat(TOTAL_LENGTH).bright_white().on_bright_black();

                    let new_version_static = "  A new version of the Pax CLI is available: ";
                    let new_version_formatted = format!("{}{}", new_version_static, new_version);
                    let new_version_line: ColoredString =
                        format!("{: <width$}", new_version_formatted, width = TOTAL_LENGTH)
                            .bright_white()
                            .on_bright_black()
                            .bold();

                    let current_version = env!("CARGO_PKG_VERSION");
                    let current_version_static = "  Currently installed version: ";
                    let current_version_formatted =
                        format!("{}{}", current_version_static, current_version);
                    let current_version_line = format!(
                        "{: <width$}",
                        current_version_formatted,
                        width = TOTAL_LENGTH
                    )
                    .bright_white()
                    .on_bright_black();

                    let update_instructions_static = "To update, run: ";
                    let lpad = (TOTAL_LENGTH - update_instructions_static.len()) / 2;
                    let lpad_spaces = " ".repeat(lpad);
                    let update_formatted = format!("{}{}", lpad_spaces, update_instructions_static);
                    let update_instructions_line =
                        format!("{: <width$}", update_formatted, width = TOTAL_LENGTH)
                            .bright_white()
                            .on_bright_black()
                            .bold();

                    let install_command_static = "cargo install --force pax-cli";
                    let lpad = (TOTAL_LENGTH - install_command_static.len()) / 2;
                    let lpad_spaces = " ".repeat(lpad);
                    let update_line_2_formatted =
                        format!("{}{}", lpad_spaces, install_command_static);
                    let update_line_2 =
                        format!("{: <width$}", update_line_2_formatted, width = TOTAL_LENGTH)
                            .bright_black()
                            .on_bright_white()
                            .bold();

                    println!();
                    println!("{}", &stars_line);
                    println!("{}", new_version_line);
                    println!("{}", current_version_line);
                    println!("{}", &empty_line);
                    println!("{}", update_instructions_line);
                    println!("{}", update_line_2);
                    println!("{}", &stars_line);
                    println!();
                }
            }
        }
    }
    if force_end_process {
        process::exit(0);
    }
}

#[cfg(unix)]
fn kill_process(pid: u32) -> Result<(), std::io::Error> {
    use std::process::Command;

    // Prefer the process group so descendants do not leak, but fall back to the
    // concrete pid for processes we did not launch in their own group.
    for target in [format!("-{}", pid), pid.to_string()] {
        let output = Command::new("kill").arg("-9").arg(&target).output()?;

        if output.status.success() {
            return Ok(());
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Failed to kill process",
    ))
}

#[cfg(windows)]
fn kill_process(pid: u32) -> Result<(), std::io::Error> {
    use std::process::Command;

    let output = Command::new("taskkill")
        .arg("/F") // forcefully kill the process
        .arg("/PID")
        .arg(pid.to_string())
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to kill process",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobile_release_run_disables_designtime_and_hot_reload() {
        for target in ["ios", "ipados", "ipad"] {
            let matches = cli()
                .get_matches_from_safe(vec![
                    "pax-cli",
                    "run",
                    "--target",
                    target,
                    "--release",
                    "--hot-reload",
                    "all",
                    "--libdev-mode",
                    "false",
                    "--ios-device",
                    "device:test-udid",
                    "--ios-development-team",
                    "TESTTEAM",
                ])
                .unwrap();
            let ctx =
                run_context(matches.subcommand_matches("run").unwrap(), Arc::default()).unwrap();
            assert!(ctx.is_release && ctx.should_also_run);
            assert!(!ctx.should_run_designtime);
            assert_eq!(ctx.hot_reload, Some(HotReloadMode::Off));
            assert_eq!(ctx.ios_device.as_deref(), Some("device:test-udid"));
            assert_eq!(ctx.ios_development_team.as_deref(), Some("TESTTEAM"));
            assert_eq!(matches!(ctx.target, RunTarget::iOS), target == "ios");
        }
    }

    #[test]
    fn debug_run_retains_designtime_and_reload_policy() {
        for lane in [None, Some("all"), Some("off")] {
            let mut argv = vec![
                "pax-cli",
                "run",
                "--target",
                "ipados",
                "--libdev-mode",
                "false",
            ];
            if let Some(lane) = lane {
                argv.extend(["--hot-reload", lane]);
            }
            let matches = cli().get_matches_from_safe(argv).unwrap();
            let ctx =
                run_context(matches.subcommand_matches("run").unwrap(), Arc::default()).unwrap();
            assert!(!ctx.is_release);
            assert!(ctx.should_run_designtime && ctx.should_also_run);
            assert_eq!(ctx.hot_reload, lane.map(|lane| lane.parse().unwrap()));
        }
    }

    #[test]
    fn release_run_does_not_enable_unvalidated_desktop_paths() {
        for target in ["web", "macos"] {
            let matches = cli()
                .get_matches_from_safe(vec!["pax-cli", "run", "--release", "--target", target])
                .unwrap();
            let error = run_context(matches.subcommand_matches("run").unwrap(), Arc::default())
                .err()
                .unwrap();
            assert!(error.to_string().contains("use build --release"));
        }
    }

    #[test]
    fn normalize_libdev_accepts_bare_flag() {
        let normalized =
            normalize_cli_args(vec!["pax-cli".into(), "build".into(), "--libdev".into()])
                .expect("bare libdev should normalize");

        assert_eq!(normalized, vec!["pax-cli", "build", "--libdev"]);
    }

    #[test]
    fn normalize_libdev_accepts_explicit_value() {
        let normalized = normalize_cli_args(vec![
            "pax-cli".into(),
            "build".into(),
            "--libdev=false".into(),
        ])
        .expect("explicit libdev should normalize");

        assert_eq!(normalized, vec!["pax-cli", "build", "--libdev-mode=false"]);
    }

    #[test]
    fn normalize_libdev_preserves_following_positional_path() {
        let normalized = normalize_cli_args(vec![
            "pax-cli".into(),
            "create".into(),
            "--libdev".into(),
            "my-project".into(),
        ])
        .expect("libdev should not consume non-mode positional values");

        assert_eq!(
            normalized,
            vec!["pax-cli", "create", "--libdev", "my-project"]
        );
    }

    #[test]
    fn format_defaults_to_current_directory() {
        let matches = cli()
            .get_matches_from_safe(vec!["pax-cli", "format"])
            .unwrap();
        let args = matches.subcommand_matches("format").unwrap();
        assert_eq!(args.value_of("format-path"), Some("."));
    }

    #[test]
    fn auto_libdev_detects_workspace_examples_and_tests_only() {
        let workspace_root = local_pax_workspace_root().expect("test should run in Pax workspace");

        assert!(auto_libdev_mode_for_project(
            &workspace_root
                .join("examples")
                .join("src")
                .join("starter-project")
        ));
        assert!(auto_libdev_mode_for_project(
            &workspace_root.join("tests").join("src").join("path-test")
        ));
        assert!(!auto_libdev_mode_for_project(
            &workspace_root.join("pax-cli")
        ));
        assert!(!auto_libdev_mode_for_project(&std::env::temp_dir()));
    }
}
