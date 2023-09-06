use std::{fs, process, thread};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use colored::Colorize;
use clap::{App, AppSettings, Arg, ArgMatches, crate_version};

use pax_compiler::{RunTarget, RunContext, CreateContext};
mod http;

use signal_hook::{iterator::Signals};
use signal_hook::consts::{SIGINT, SIGTERM};


fn main() -> Result<(), ()> {

    let current_version = env!("CARGO_PKG_VERSION");

    

    //Shared state to store child processes keyed by static unique string IDs, for cleanup tracking
    let process_child_ids: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(vec![]));
    // Shared state to store the new version info if available.
    let new_version_info = Arc::new(Mutex::new(None));

    // Spawn the check_for_update thread so it runs concurrently.
    let cloned_new_version_info = Arc::clone(&new_version_info);
    thread::spawn(move || {
        http::check_for_update(current_version, cloned_new_version_info);
    });

    // Create a separate thread to handle signals e.g. via CTRL+C
    let mut signals = Signals::new(&[SIGINT, SIGTERM]).unwrap();
    let cloned_version_info = Arc::clone(&new_version_info);
    let cloned_process_child_ids = Arc::clone(&process_child_ids);
    thread::spawn(move || {
        for _sig in signals.forever() {
            perform_cleanup(Arc::clone(&cloned_version_info),Arc::clone(&cloned_process_child_ids));
        }
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

    let _ = perform_nominal_action(matches, Arc::clone(&process_child_ids));
    perform_cleanup(new_version_info, process_child_ids);

    Ok(())
}


fn perform_cleanup(new_version_info: Arc<Mutex<Option<String>>>, process_child_ids: Arc<Mutex<Vec<u64>>> ) {
    const RETRY_COUNT : u8 = 2;
    const RETRY_PERIOD_MS : u64 = 250;
    let mut current_count : u8 = 0;

    //1. kill any running child processes
    while current_count < RETRY_COUNT {
        if let Ok(process_child_ids_lock) = process_child_ids.lock() {
            process_child_ids_lock.iter().for_each(|child_id| {
                kill_process(*child_id as u32).expect(&format!("Failed to kill process with ID: {}", child_id));
            });
            break;
        } else {
            current_count = current_count + 1;
            thread::sleep(Duration::from_millis(RETRY_PERIOD_MS));
        }
    }

    //2. print update message if appropriate

    let mut current_count : u8 = 0;
    while current_count < RETRY_COUNT {
        if let Ok(new_version_lock) = new_version_info.lock() {
            if let Some(new_version) = new_version_lock.as_ref() {
                println!();
                println!("************************************************************");
                println!("{}", format!("A new version of the Pax CLI is available: {}", new_version).blue().bold());
                println!("To update, run: `cargo install --force pax-cli`");
                println!("************************************************************");
                println!();
            }
            break;
        } else {
            current_count = current_count + 1;
            thread::sleep(Duration::from_millis(RETRY_PERIOD_MS));
        }
    }

    process::exit(0);
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
        Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to kill process"))
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
        Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to kill process"))
    }
}


fn perform_nominal_action(matches: ArgMatches<'_>, process_child_ids: Arc<Mutex<Vec<u64>>>) -> Result<(), ()> {
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
                process_child_ids,
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
                process_child_ids,
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
                    let output = &pax_compiler::run_parser_binary(&path, process_child_ids);

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

                    let output = pax_compiler::build_chassis_with_cartridge(&pax_dir, &RunTarget::from(target.as_str()), process_child_ids);

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