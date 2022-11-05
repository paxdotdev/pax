


#[tokio::main]
async fn main() -> Result<(), Error> {
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
                .arg(
                    Arg::with_name("path")
                        .short("p")
                        .long("path")
                        .takes_value(true)
                        .default_value(".")
                )
                .arg(
                    Arg::with_name("target")
                        .short("t")
                        .long("target")
                        //Default to web -- perhaps the ideal would be to discover host
                        //platform and run appropriate native harness.  Web is a suitable,
                        //sane default for now.
                        .default_value("web")
                        .help("Specify the target platform on which to run.  Will run in platform-specific demo harness.")
                        .takes_value(true),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("run", Some(args)) => {

            let target = args.value_of("target").unwrap().to_lowercase();
            let path = args.value_of("path").unwrap().to_string(); //default value "."

            perform_run(RunContext{
                target: RunTarget::from(target.as_str()),
                path,
                handle: Handle::current(),
            }).await?;

        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }

    Ok(())
}
