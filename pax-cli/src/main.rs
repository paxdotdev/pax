use clap::{crate_version, App, AppSettings, Arg, ArgMatches};
use color_eyre::config::HookBuilder;
use colored::{ColoredString, Colorize};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{process, thread};

use pax_compiler::{CreateContext, RunContext, RunTarget};
extern crate pax_language_server;

mod http;

use color_eyre::eyre::Report;
use color_eyre::eyre::Result;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

/// `pax-cli` entrypoint
fn main() -> Result<(), Report> {
    HookBuilder::default()
        .display_location_section(false)
        .install()?;

    //Shared state to store child processes keyed by static unique string IDs, for cleanup tracking
    let process_child_ids: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(vec![]));
    // Shared state to store the new version info if available.
    let new_version_info = Arc::new(Mutex::new(None));

    // Spawn the check_for_update thread so it runs concurrently.
    let cloned_new_version_info = Arc::clone(&new_version_info);
    thread::spawn(move || {
        http::check_for_update(cloned_new_version_info);
    });

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
        .help("Specify the target platform on which to run.  Will run in platform-specific demo harness.")
        .takes_value(true);

    #[allow(non_snake_case)]
    let ARG_RELEASE = Arg::with_name("release")
        .long("release")
        .takes_value(false)
        .help("Build in Release mode, with appropriate platform-specific optimizations.");

    #[allow(non_snake_case)]
    let ARG_LIBDEV = Arg::with_name("libdev")
        .long("libdev")
        .takes_value(false)
        .help("Signal to the compiler to run certain operations in libdev mode, offering certain ergonomic affordances for Pax library developers.")
        .hidden(true); //hidden because this is of negative value to end-users; things are expected to break when invoked outside of the pax monorepo

    #[allow(non_snake_case)]
    let ARG_LSP = App::new("lsp").about("Start the Pax LSP server");

    let matches = App::new("pax")
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
                .arg( ARG_VERBOSE.clone() )
                .arg( ARG_LIBDEV.clone() )
        )
        .subcommand(
            App::new("build")
                .about("Builds the Pax project from the current working directory into a platform-specific executable, for the specific `target` platform.")
                .arg( ARG_PATH.clone() )
                .arg( ARG_TARGET.clone() )
                .arg( ARG_VERBOSE.clone() )
                .arg( ARG_LIBDEV.clone() )
                .arg( ARG_RELEASE.clone() )
        )
        .subcommand(
            App::new("clean")
                .arg( ARG_PATH.clone() )
                .arg( ARG_LIBDEV.clone() )
                .about("Cleans the temporary files associated with the Pax project in the current working directory — notably, the temporary files generated into the .pax directory")
        )
        .subcommand(
            App::new("create")
                .about("Creates a new Pax + Rust project at the specified path, including necessary boilerplate and default configuration.")
                .alias("new")
                .arg(Arg::with_name("path")
                    .help("File system path where the new project should be created. If not provided with --path, it should directly follow 'create'")
                    .takes_value(true)
                    .index(1))  // Positional arg, `pax create positional_arg_here`
                .arg( ARG_LIBDEV.clone())
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
        .subcommand(ARG_LSP.clone())
        .get_matches();

    // Clap doesn't easily let us check a "global" arg without performing individual `match`es.
    // Since we want to know at this top level whether `--libdev` is present, we will parse it manually.
    let args: Vec<String> = std::env::args().collect();
    let is_libdev_mode = args.contains(&"--libdev".to_string());

    // Create a separate thread to handle signals e.g. via CTRL+C
    let mut signals = Signals::new(&[SIGINT, SIGTERM]).unwrap();
    let cloned_version_info = Arc::clone(&new_version_info);
    let cloned_process_child_ids = Arc::clone(&process_child_ids);
    thread::spawn(move || {
        for _sig in signals.forever() {
            println!("\nInterrupt received. Cleaning up child processes...");
            perform_cleanup(
                Arc::clone(&cloned_version_info),
                Arc::clone(&cloned_process_child_ids),
                is_libdev_mode,
                true,
            );
        }
    });

    let res = perform_nominal_action(matches, Arc::clone(&process_child_ids));
    perform_cleanup(new_version_info, process_child_ids, is_libdev_mode, false);
    res
}

fn perform_nominal_action(
    matches: ArgMatches<'_>,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
) -> Result<(), Report> {
    match matches.subcommand() {
        ("run", Some(args)) => {
            let target = args.value_of("target").unwrap().to_lowercase();
            let path = args.value_of("path").unwrap().to_string(); //default value "."
            let verbose = args.is_present("verbose");
            let is_libdev_mode = args.is_present("libdev");

            pax_compiler::perform_build(&RunContext {
                target: RunTarget::from(target.as_str()),
                path,
                verbose,
                should_also_run: true,
                is_libdev_mode,
                process_child_ids,
                is_release: false,
            })
        }
        ("build", Some(args)) => {
            let target = args.value_of("target").unwrap().to_lowercase();
            let path = args.value_of("path").unwrap().to_string(); //default value "."
            let verbose = args.is_present("verbose");
            let is_libdev_mode = args.is_present("libdev");
            let is_release = args.is_present("release");

            pax_compiler::perform_build(&RunContext {
                target: RunTarget::from(target.as_str()),
                path,
                should_also_run: false,
                verbose,
                is_libdev_mode,
                process_child_ids,
                is_release,
            })
        }
        ("clean", Some(args)) => {
            println!("🧹 Cleaning cached & temporary files...");
            let path = args.value_of("path").unwrap().to_string(); //default value "."

            pax_compiler::perform_clean(&path);
            thread::sleep(Duration::from_millis(1000)); //Sleep for 1s to let update check finish

            println!("Done.");
            Ok(())
        }
        ("create", Some(args)) => {
            let path = args.value_of("path").unwrap().to_string(); //default value "."
            let is_libdev_mode = args.is_present("libdev");
            let version = crate_version!().to_string(); // Note: this could also be parameterized, but an easy default is to clamp to the CLI version

            pax_compiler::perform_create(&CreateContext {
                path,
                is_libdev_mode,
                version,
            });
            Ok(())
        }
        ("libdev", Some(args)) => {
            match args.subcommand() {
                ("parse", Some(args)) => {
                    let path = args.value_of("path").unwrap().to_string(); //default value "."
                    let output = &pax_compiler::run_parser_binary(&path, process_child_ids);

                    // Forward both stdout and stderr
                    std::io::stderr()
                        .write_all(output.stderr.as_slice())
                        .unwrap();
                    std::io::stdout()
                        .write_all(output.stdout.as_slice())
                        .unwrap();

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
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
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

    // Use the negative PID to refer to the process group
    let output = Command::new("kill")
        .arg("-9") // send SIGKILL
        .arg(format!("-{}", pid))
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

#[cfg(windows)]
fn kill_process(pid: u64) -> Result<(), std::io::Error> {
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
