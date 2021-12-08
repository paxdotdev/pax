use clap::{App, AppSettings, Arg};

fn main() {
    let matches = App::new("pax")
        .about("Pax language compiler and dev tooling")
        .version("0.0.1")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .author("Zack Brown")
        // Query subcommand
        //
        // Only a few of its arguments are implemented below.
        .subcommand(
            App::new("run")
                .about("Run the Pax project in the current working directory")
                .arg(
                    Arg::with_name("target")
                        .short("t")
                        .long("target")
                        .help("Specify the target platform on which to run ")
                        .takes_value(true),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("run", Some(sync_matches)) => {
            if sync_matches.is_present("target") {
                unimplemented!("Target currently hard-coded for web")
            }

            println!("Run logic here")

        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}



