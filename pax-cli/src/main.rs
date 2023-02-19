use std::fs;
use std::io::Write;
use std::path::Path;
use clap::{App, AppSettings, Arg};
use pax_compiler::{RunTarget, RunContext};




fn main() -> Result<(), ()> {

    #[allow(non_snake_case)]
    let ARG_PATH = Arg::with_name("path")
        .short("p")
        .long("path")
        .takes_value(true)
        .default_value(".");

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

    let matches = App::new("pax")
        .name("pax")
        .bin_name("pax")
        .about("Pax CLI including compiler and dev tooling")
        .version("0.0.1")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .author("Zack Brown <zack@pax.rs>")
        .subcommand(
            App::new("run")
                .about("Run the Pax project from the current working directory in a demo harness")
                .arg( ARG_PATH.clone() )
                .arg( ARG_TARGET.clone() ),
        )
        .subcommand(
            App::new("build")
                .about("Builds the Pax project from the current working directory into a platform-specific executable, for the specific `target` platform.")
                .arg( ARG_PATH.clone() )
                .arg( ARG_TARGET.clone() ),
        )
        .subcommand(
            App::new("clean")
                .arg( ARG_PATH.clone() )
                .about("Cleans the temporary files associated with the Pax project in the current working directory — notably, the temporary files generated into the .pax directory")
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

    match matches.subcommand() {
        ("run", Some(args)) => {

            let target = args.value_of("target").unwrap().to_lowercase();
            let path = args.value_of("path").unwrap().to_string(); //default value "."

            pax_compiler::perform_build(&RunContext{
                target: RunTarget::from(target.as_str()),
                path,
            }, true)

        },
        ("build", Some(args)) => {
            let target = args.value_of("target").unwrap().to_lowercase();
            let path = args.value_of("path").unwrap().to_string(); //default value "."

            pax_compiler::perform_build(&RunContext{
                target: RunTarget::from(target.as_str()),
                path,
            }, false)
        },
        ("clean", Some(args)) => {
            let path = args.value_of("path").unwrap().to_string(); //default value "."

            pax_compiler::perform_clean(&path)
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
                _ => {unreachable!()}
            }
        },
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }

}
