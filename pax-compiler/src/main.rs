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
        ("run", Some(args)) => {
            if args.is_present("target") {
                unimplemented!("Target currently hard-coded for web")
            }

            println!("Run logic here")
            //1. compile project with Cargo — yields a lib ("cartridge") plus `designer` bindings — note: no Pax yet
            //2. `patch` cartridge into chassis and build native lib (e.g. .wasm filem — starting with Web in the following exploration)
            //** PROBLEM: at this point, e.g. with wasm, the browser is the host for the
            //   entire program — meaning that hosting an HTTP server (in the browser, via wasm) is a no-go
            //   That said, web-sockets might work...
            //   Another option: debug using a native chassis, which could expose an HTTP
            //   server in the same process without browser sandbox hurdles
            //   Another option (maybe MVP) — parse Pax headlessly; transpile to RIL and compile
            //   to wasm in order to view in browser (rules out live updates)

            //  Major options seems to be (a) desktop/native renderer + web server, or
            //                            (b) websocket/webrtc comms from browser
            //        Browser surfaces several problems:
            //          1.  ability to host the HTTP server
            //              (could be worked out with websockets + hacks)
            //          2.  fs access (e.g. to write back to RIL)
            //              (could be delegated back to compiler process — wasm process can yield strings, which compiler/designer process writes to FS)
            //          3.  calling `cargo`/`rustc`, and more...
            //              (could be handled by compiler/host process)
            //        At the same time, relying on a native renderer + process would dead-end
            //        us from supporting "live design" in the browser.
            //
            //3. Phone home from wasm to compiler-server, to establish duplex connection via websocket (+ auth token, keep-alive)
            //4. Parse token pairs from .pax, feed them to .wasm process
            //
            //2. build project into chassis, e.g. for web, build into template project
            //3. fire up chassis

        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}



