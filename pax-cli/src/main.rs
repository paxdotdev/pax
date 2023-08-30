use std::fs;
use std::io::Write;
use std::path::Path;
use std::str::Matches;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::signal;
use colored::Colorize;
use clap::{App, AppSettings, Arg, ArgMatches, crate_version};
use tokio::signal::unix::{Signal, SignalKind};
use pax_compiler::{RunTarget, RunContext, CreateContext};

mod http;

#[tokio::main]
async fn main() -> Result<(), ()> {

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

    #[allow(non_snake_case)]
    let ARG_NAME = Arg::with_name("name")
        .long("name")
        .takes_value(true)
        .required(true)
        .help("Name for the new Pax project.  Will be used in multiple places, including the name of the crate for Cargo and the name of the directory where the project is generated.");

    #[allow(non_snake_case)]
    let ARG_TARGET = Arg::with_name("target")
        .short("t")
        .long("target")
        //Default to web -- perhaps the ideal would be to discover host
        //platform and run appropriate native harness.  Web is a suitable,
        //sane default for now.
        .default_value("web")
        .help("Specify the target platform on which to run.  Will run in platform-specific demo harness.")
        .takes_value(true);

    #[allow(non_snake_case)]
    let ARG_LIBDEV = Arg::with_name("libdev")
        .long("libdev")
        .takes_value(false)
        .help("Signal to the compiler to run certain operations in libdev mode, offering certain ergonomic affordances for Pax library developers.")
        .hidden(true); //hidden because this is of negative value to end-users; things are expected to break when invoked outside of the pax monorepo


    let matches = App::new("pax")
        .name("pax")
        .bin_name("pax")
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
        )
        .subcommand(
            App::new("clean")
                .arg( ARG_PATH.clone() )
                .about("Cleans the temporary files associated with the Pax project in the current working directory — notably, the temporary files generated into the .pax directory")
        )
        .subcommand(
            App::new("create")
                .alias("new")
                .arg( ARG_PATH.clone() )
                .arg( ARG_LIBDEV.clone() )
                .arg(ARG_NAME.clone() )
                .about("Creates a new Pax project in a new directory with the specified `name`.  If a `path` is specified, the new directory `name` will be appended to the `path`.")
        )
        .subcommand(
            App::new("libdev")
                .subcommand(
                    App::new("build-chassis")
                        .arg( ARG_PATH.clone() )
                        .arg( ARG_TARGET.clone() )
                        .about("Runs cargo build on the codegenned chassis, within the .pax folder contained within the specified `path`.  Useful for core development, e.g. building compiler features or compiler debugging.  Expected to fail if the whole compiler has not run at least once.")
                )
                .subcommand(
                    App::new("parse")
                        .arg( ARG_PATH.clone() )
                        .about("Parses the Pax program at the specified path and prints the manifest object, serialized to string. Also prints error messages if parsing fails.")
                )
                .about("Collection of tools for internal library development")
        )
        .get_matches();

    let current_version = env!("CARGO_PKG_VERSION");

    // Shared state to store the new version info if available.
    let new_version_info = Arc::new(Mutex::new(None));

    // Spawn the check_for_update task so it runs concurrently.
    let update_check_handle = tokio::spawn(crate::http::check_for_update(current_version, new_version_info.clone()));

    // Use tokio::select! to wait for either the nominal action to complete or the interrupt signal.
    tokio::select! {
        _ = perform_nominal_action(matches) => {}
        _ = wait_for_signals() => {}
    }

    // After the primary action is done, check if there was an update info available.
    if let Some(new_version) = new_version_info.lock().await.as_ref().cloned() {
        println!();
        println!("************************************************************");
        println!("{}", format!("A new version of the Pax CLI is available: {}", new_version).blue().bold());
        println!("To update, run: `cargo install --force pax-cli`");
        println!("************************************************************");
        println!();
    }

    Ok(())
}

async fn wait_for_signals() {
    #[cfg(unix)]
    {
        let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt()).unwrap();
        let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate()).unwrap();

        tokio::select! {
            _ = sigint.recv() => {
                println!("Received SIGINT. Cleaning up...");
            }
            _ = sigterm.recv() => {
                println!("Received SIGTERM. Cleaning up...");
            }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
        println!("Received Ctrl+C. Cleaning up...");
    }
}

async fn perform_nominal_action(matches: ArgMatches<'_>) -> Result<(), ()> {
    match matches.subcommand() {
        ("run", Some(args)) => {
            let target = args.value_of("target").unwrap().to_lowercase();
            let path = args.value_of("path").unwrap().to_string(); //default value "."
            let verbose = args.is_present("verbose");
            let libdevmode = args.is_present("libdev");

            pax_compiler::perform_build(&RunContext {
                target: RunTarget::from(target.as_str()),
                path,
                verbose,
                should_also_run: true,
                libdevmode,
            })
        },
        ("build", Some(args)) => {
            let target = args.value_of("target").unwrap().to_lowercase();
            let path = args.value_of("path").unwrap().to_string(); //default value "."
            let verbose = args.is_present("verbose");
            let libdevmode = args.is_present("libdev");

            pax_compiler::perform_build(&RunContext {
                target: RunTarget::from(target.as_str()),
                path,
                should_also_run: false,
                verbose,
                libdevmode,
            })
        },
        ("clean", Some(args)) => {
            let path = args.value_of("path").unwrap().to_string(); //default value "."

            pax_compiler::perform_clean(&path);
            Ok(())
        },
        ("create", Some(args)) => {
            let path = args.value_of("path").unwrap().to_string(); //default value "."
            let name = args.value_of("name").unwrap().to_string(); //default value "."
            let libdevmode = args.is_present("libdev");
            let version = crate_version!().to_string(); // Note: this could also be parameterized, but an easy default is to clamp to the CLI version

            pax_compiler::perform_create(&CreateContext {
                crate_name: name,
                path,
                libdevmode,
                version,
            });
            Ok(())
        },
        ("libdev", Some(args)) => {
            match args.subcommand() {
                ("parse", Some(args)) => {
                    let path = args.value_of("path").unwrap().to_string(); //default value "."
                    let output = &pax_compiler::run_parser_binary(&path);

                    // Forward both stdout and stderr
                    std::io::stderr().write_all(output.stderr.as_slice()).unwrap();
                    std::io::stdout().write_all(output.stdout.as_slice()).unwrap();

                    Ok(())
                },
                ("build-chassis", Some(args)) => {
                    let target = args.value_of("target").unwrap().to_lowercase();
                    let path = args.value_of("path").unwrap().to_string(); //default value "."

                    let working_path = Path::new(&path).join(".pax");
                    let pax_dir = fs::canonicalize(working_path).unwrap();

                    let output = pax_compiler::build_chassis_with_cartridge(&pax_dir, &RunTarget::from(target.as_str()));

                    // Forward both stdout and stderr
                    std::io::stderr().write_all(output.stderr.as_slice()).unwrap();
                    std::io::stdout().write_all(output.stdout.as_slice()).unwrap();

                    Ok(())
                },
                _ => { unreachable!() }
            }
        },
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}