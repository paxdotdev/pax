
use clap::{App, AppSettings, Arg, Error};
use pax_compiler::{RunTarget, RunContext};




fn main() -> Result<(), ()> {

    let ARG_PATH = Arg::with_name("path")
        .short("p")
        .long("path")
        .takes_value(true)
        .default_value(".");

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
        .author("Zack Brown <zack@pax-lang.org>")
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
                .about("Cleans the temporary files associated with the Pax project in the current working directory — notably, the temporary files generated into the .pax directory")
        )
        .get_matches();

    match matches.subcommand() {
        ("run", Some(args)) => {

            let target = args.value_of("target").unwrap().to_lowercase();
            let path = args.value_of("path").unwrap().to_string(); //default value "."

            pax_compiler::perform_build(RunContext{
                target: RunTarget::from(target.as_str()),
                path,
            }, true)

        },
        ("build", Some(args)) => {
            let target = args.value_of("target").unwrap().to_lowercase();
            let path = args.value_of("path").unwrap().to_string(); //default value "."

            pax_compiler::perform_build(RunContext{
                target: RunTarget::from(target.as_str()),
                path,
            }, false)
        },
        ("clean", _) => {
            unimplemented!()
        },
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }

}
