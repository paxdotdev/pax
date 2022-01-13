#[macro_use]
extern crate pest_derive;

use tokio::net::{TcpListener, TcpStream};

mod parser;
mod server;

use std::{thread::{Thread, self}, sync::mpsc::{self, Receiver, Sender}, time::Duration, process::{Command, Stdio}};

use clap::{App, AppSettings, Arg};


#[tokio::main]
async fn main() -> Result<(), ()> {
    let matches = App::new("pax")
        .name("pax")
        .bin_name("pax")
        .about("Pax language compiler and dev tooling")
        .version("0.0.1")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .author("Zack Brown <zack@inclination.co>")
        .subcommand(
            App::new("run")
                .about("Run the Pax project from the current working directory in a demo harness")
                .arg(
                    Arg::with_name("path")
                        .takes_value(true)
                        .default_value(".")
                        .index(1)
                )
                .arg(
                    Arg::with_name("target")
                        .short("t")
                        .long("target")
                        .help("Specify the target platform on which to run.  Will run in platform-specific demo harness.")
                        .takes_value(true),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("run", Some(args)) => {
            let target = "web";

            if args.is_present("target") {
                unimplemented!("Target currently hard-coded for web")
            }

            let path = args.value_of("path").unwrap();

            perform_run(target, path);

            println!("Run logic here: {}", path);

        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }

    Ok(())
}






#[derive(Default)]
struct ThreadWrapper<T> {
    handle: Option<thread::JoinHandle<u8>>,
    sender: Option<Sender<T>>,
    receiver: Option<Receiver<T>>,
    red_phone: Option<Sender<T>>,
}

struct ProcessWrapper<T> {
    handle: Option<thread::JoinHandle<u8>>,
    sender: Option<Sender<T>>,
    receiver: Option<Receiver<T>>,
    red_phone: Option<Sender<T>>,
}

// fn start_thread_macro_coordination() -> ThreadWrapper<MessageMacroCoordination> {

//     let (tx_out, rx_out) = mpsc::channel();
//     let (tx_in, rx_in) = mpsc::channel();
//     let (tx_red, rx_red) = mpsc::channel();

//     let handle = thread::spawn(move || {


//         //set up HTTP or WS server — 
//         let vals = vec![
//             String::from("hi"),
//             String::from("from"),
//             String::from("the"),
//             String::from("thread"),
//         ];

//         for val in vals {
//             tx1.send(val).unwrap();
//             thread::sleep(Duration::from_secs(1));
//         }
//     });




//     let mut thread_wrapper  = ThreadWrapper{
//         handle,
//         receiver: (),
//         red_phone: (),
//         sender: (),
//     };

//     // thread_wrapper.handle
// }


fn get_open_tcp_port() -> u16 {
    const RANGE_START : u16 = 4242;
    let current = RANGE_START;
    while !portpicker::is_free_tcp(current) {
        current = current + 1;
    }
    current
}



// struct MessageMacroCoordination {}
struct MessageCargo {}
struct MessageForwarder {}

#[derive(Default)]
struct RunContext {
    // ThreadMacroCoordination: Option<ThreadWrapper<MessageMacroCoordination>>,
    ProcessCargo: Option<ProcessWrapper<MessageCargo>>,
    ThreadForwarder: Option<ThreadWrapper<MessageForwarder>>,
}

/// Run the project at the specified path inside the demo chassis
/// for the specified target platform
fn perform_run(target: &str, path: &str) {
    let mut ctx = RunContext::default();
    //see pax-compiler-sequence-diagram.png


    // ctx.ThreadMacroCoordination = Some(start_thread_macro_coordination());


    // Instead of the full macro coordination server, try a simpler approach:
    // "every macro writes to a file", waiting for the file to be ready as long as necessary to write.
    //  Note that the file lock management becomes a little tricky.

    // What about OS pipes?  the message passing is very simple — no risk of
    // deadlocks as long as the message-passing is one-way
    
    let macro_coordination_tcp_port= get_open_tcp_port();

    ctx.ProcessCargo = Some(start_cargo_process(macro_coordination_tcp_port));


    let thread_join_handle = thread::spawn(move || {
        // some work here
    });

    let (tx, rx) = mpsc::channel();

    let tx1 = tx.clone();
    thread::spawn(move || {
        let vals = vec![
            String::from("hi"),
            String::from("from"),
            String::from("the"),
            String::from("thread"),
        ];

        for val in vals {
            tx1.send(val).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    });

    thread::spawn(move || {
        let vals = vec![
            String::from("more"),
            String::from("messages"),
            String::from("for"),
            String::from("you"),
        ];

        for val in vals {
            tx.send(val).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    });

    for received in rx {
        println!("Got: {}", received);
    }



    //TODO: start macro coordination server
    ctx.MacroCoordinationThread = Some(macro_coordination::start_server());
    /*
    Option A: dump to an append-only file; load that file after compilation
    Option B: open a simple HTTP server 
    */


    ctx.MacroCoordinationThread.unwrap().attach_listener("finish", |data| {
        //use the dumped data gathered by macros:
        // - location of all pax files to create a work queue for parsing
        // - paths to import for PropertiesCoproduct members, to code-gen PropertiesCoproduct and its Cargo.toml
    });


    //TODO: start cargo build
    // ctx.CargoBuildThread = Some(start_cargo_build_thread())    


    //Await completion of both threads
    
    //TODO: perform any necessary codegen, incl. patched Cargo.toml, into temp dir
    //TODO: run cargo build again; generate .wasm (or other platform-native lib)
    //TODO: start websocket server
    //TODO: start demo harness, load cartridge
    //TODO: establish duplex connection to WS server from cartridge
    //TODO: start parsing Pax files


}



fn start_cargo_process(macro_coordination_tcp_port: u16) -> ProcessWrapper<MessageCargo> {
    
    let process = match Command::new("wc")
                                .stdin(Stdio::piped())
                                .stdout(Stdio::piped())
                                .spawn() {
        Err(why) => panic!("couldn't spawn wc: {}", why),
        Ok(process) => process,
    };

    unimplemented!()
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

