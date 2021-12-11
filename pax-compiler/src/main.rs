#[macro_use]
extern crate pest_derive;

use std::sync::mpsc;
use std::{thread, time};
use std::time::Duration;
use actix_web::dev::Server;
use actix_web::{HttpServer, middleware, rt, web, App as ActixWebApp};


mod parser;
mod server;

use clap::{App, AppSettings, Arg};

fn main() {
    let matches = App::new("pax")
        .name("pax")
        .bin_name("pax")
        .about("Pax language compiler and dev tooling")
        .version("0.0.1")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .author("Zack Brown <zack@inclination.co>")
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

            println!("Run logic here");
            //1. compile project with Cargo — yields a lib ("cartridge") plus `designtime` extensions — note: no Pax yet
            //  [ ]
            //2. `patch` cartridge into chassis and build native lib (e.g. .wasm file — starting with Web in this pass)
            //3. Start websocket server
            start_ws_server();
            //4. Mount the compiled cartridge+chassis+designtime into a "demo app," e.g. for web an index.html + js mount of the wasm file (see pax-chassis-web for model)
            //5. From running sample app: phone home from wasm to compiler — via chassis, since ws client connection method is a platform-specific concern — to establish duplex connection (+ auth token, keep-alive mechanism)
            //6. From compiler [this] process: parse token pairs from .pax, feed them to .wasm process (accept token pairs over websockets and call wasm-local ORM CRUD methods)
            //

        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}


fn start_ws_server() {
    std::env::set_var("RUST_LOG", "actix_web=info,actix_server=trace");
    // env_logger::init();

    let (tx, rx) = mpsc::channel();

    println!("START SERVER");
    thread::spawn(move || {
        let _ = start_ws_threaded(tx);
    });

    let srv = rx.recv().unwrap();

    println!("WAITING 10 SECONDS");
    thread::sleep(time::Duration::from_secs(10));

    println!("STOPPING SERVER");
    // init stop server and wait until server gracefully exit
    rt::System::new("").block_on(srv.stop(true));
}

fn start_ws_threaded(tx: mpsc::Sender<Server>) -> std::io::Result<()> {
    let mut sys = rt::System::new("test");

    // srv is server controller type, `dev::Server`
    let srv = HttpServer::new(|| {
        ActixWebApp::new()
            // enable logger
            .wrap(middleware::Logger::default())
            .service(web::resource("/index.html").to(|| async { "Hello world!" }))
            // .service(web::resource("/").to(index))
    })
        .bind("127.0.0.1:8080")?
        .run();

    // send server controller to main thread
    let _ = tx.send(srv.clone());

    // run future
    sys.block_on(srv)
}


// Appendix
//** PROBLEM: at this point, e.g. with wasm, the browser is the host for the
//    entire program — meaning that hosting an HTTP server (in the browser, via wasm) is a no-go
//    That said, web-sockets might work...
//    Another option: debug using a native chassis, which could expose an HTTP
//    server in the same process without browser sandbox hurdles
//    Another option (maybe MVP) — parse Pax headlessly; transpile to RIL and compile
//    to wasm in order to view in browser (rules out live updates)
//
//   Major options seems to be (a) desktop/native renderer + web server, or
//                             (b) websocket/webrtc comms from browser
//         Browser surfaces several problems:
//           1.  ability to host the HTTP server
//               (could be worked out with websockets + hacks)
//           2.  fs access (e.g. to write back to RIL)
//               (could be delegated back to compiler process — wasm process can yield strings, which compiler/designtime process writes to FS)
//           3.  calling `cargo`/`rustc`, and more...
//               (could be handled by compiler/host process)
//         At the same time, relying on a native renderer + process would dead-end
//         us from supporting "live design" in the browser.

