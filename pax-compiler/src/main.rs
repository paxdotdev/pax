

use tokio::net::{TcpListener, TcpStream};

use tokio::task::yield_now;
use tokio::{select, task};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{Sender, Receiver, UnboundedReceiver};
use tokio_stream::wrappers::{ReceiverStream};



mod parser;
mod server;

use std::io::Error;
use std::task::{Poll, Context};
use std::{thread::{Thread, self}, time::Duration};
use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::sync::Arc;

use clap::{App, AppSettings, Arg};

use futures::prelude::*;



use pax_message::PaxMessage;
use serde_json::Value;
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio_serde::SymmetricallyFramed;
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};

// use serde_json::Value;
use tokio_serde::formats::*;
// use tokio_util::codec::{FramedRead, LengthDelimitedCodec};


#[tokio::main]
async fn main() -> Result<(), Error> {
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

            perform_run(RunContext{
                target: target.into(), 
                path: path.into(), 
                handle: Handle::current(),
            }).await?;


        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }

    Ok(())
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
    //TODO: mitigate races within this process where 
    //      get_open_tcp_port is called in quick succession.
    //      Could keep a simple hashmap "burn list", ensuring that not only
    //      is a port open per the OS, but it's also not in the burn list.
    const RANGE_START : u16 = 4242;
    let mut current = RANGE_START;
    while !portpicker::is_free_tcp(current) {
        current = current + 1;
    }
    current
}



// struct MessageMacroCoordination {}
struct MessageCargo {}
struct MessageForwarder {}

struct RunContext {
    target: String,
    path: String,
    handle: Handle,
    // ThreadMacroCoordination: Option<ThreadWrapper<MessageMacroCoordination>>,
}


#[derive(Debug)]
struct ComponentManifest {
    pax_file_path: String,
}

async fn run_macro_coordination_server(mut red_phone: UnboundedReceiver<bool>, return_data_channel : tokio::sync::oneshot::Sender<Vec<ComponentManifest>>, macro_coordination_tcp_port: u16) -> Result<(), Error> {

    // Bind a server socket
    let listener = TcpListener::bind(format!("127.0.0.1:{}",macro_coordination_tcp_port)).await.unwrap();
    let mut manifests: Vec<ComponentManifest> = vec![];

    loop {
        tokio::select! {
            _ = red_phone.recv() => {
                //for now, any message from parent is the shutdown message
                println!("Red phone message received");
                &return_data_channel.send(manifests);
                break;
            }
            _ = listener.accept() => {
                println!("TCP message received");
                &manifests.push(ComponentManifest{pax_file_path: "TODO: get from TCP frame".into()});
            }
        }


        //
        //
        //
        // let (socket, _) = future::select(red_phone.blocking_recv().await, listener.accept().await).unwrap();
        // // let (socket, _) = listener.accept().await.unwrap();
        //
        // // Delimit frames using a length header
        // let length_delimited = FramedRead::new(socket, LengthDelimitedCodec::new());
        //
        // // Deserialize frames
        // let mut deserialized = tokio_serde::SymmetricallyFramed::new(
        //     length_delimited,
        //     SymmetricalJson::<Value>::default(),
        // );
        //
        // // Spawn a task that prints all received messages to STDOUT
        // tokio::spawn(async move {
        //     while let Some(msg) = deserialized.try_next().await.unwrap() {
        //         println!("GOT: {:?}", msg);
        //     }
        // });



    }


    // let listener = TcpListener::bind(
    //     format!("127.0.0.1:{}", macro_coordination_tcp_port.to_string())
    // ).await?;

    // let mut empty_context = Context::from(_)
    // loop {
    //     match listener.poll_accept(&mut empty_context) {
    //         Poll::Ready(result) => {
    //             //process incoming data
    //             // result.unwrap().0
    //             print!("received TCP data");

    //         },
    //         _ => {},
    //     }

    //     match red_phone.poll_recv(&mut empty_context) {
    //         Poll::Ready(msg) => {
    //             //for now, any message from parent is the shutdown message
    //             break;
    //         },
    //         Poll::Pending => {},
    //     }        
    // }

    Ok(())
}

/// Run the project at the specified path inside the demo chassis
/// for the specified target platform
async fn perform_run(ctx: RunContext) -> Result<(), Error> {
    
    //see pax-compiler-sequence-diagram.png


    let macro_coordination_tcp_port= get_open_tcp_port();
    println!("Listening for macro communication on open TCP port: {}", macro_coordination_tcp_port);

    let (red_phone_tx, red_phone_rx) = tokio::sync::mpsc::unbounded_channel();
    let (payload_tx, payload_rx) = tokio::sync::oneshot::channel();

    let handle = task::spawn(
        run_macro_coordination_server(red_phone_rx, payload_tx, macro_coordination_tcp_port)
    );

    let cargo_future = Command::new("cargo").current_dir(ctx.path).arg("build")
        .spawn().expect("failed to execute cargo build").wait_with_output();

    let mut cargo_exit_code : i32 = -1;

    select! {
        _ = handle => {
            panic!("macro coordination server failed"); //this should not return before cargo does; should require manual shutdown
        }
        exit_code = cargo_future => {
            cargo_exit_code = exit_code.expect("failed to capture exit code").status.code().unwrap();
        }
    }

    //TODO: cp lib.rs to .pax.manifest.rs

    //TODO: clean up .pax.manifest.rs

    println!("Waiting 15 seconds for messages...");
    thread::sleep(Duration::from_secs(15));

    println!("Sending shutdown signal");
    red_phone_tx.send(true); //send shutdown signal

    let component_manifest = payload_rx.await.expect("failed to retrieve component_manifest from macro coordination thread");

    println!("received component manifest: {:?}",component_manifest);
    // handle.await?;

    

    // listener.

    // let server = listener.incoming().for_each(move |socket| {
    //     // TODO: Process socket
    //     Ok(())
    // })
    // .map_err(|err| {
    //     // Handle error by printing to STDOUT.
    //     println!("accept error = {:?}", err);
    // });



    // // Instead of the full macro coordination server, try a simpler approach:
    // // "every macro writes to a file", waiting for the file to be ready as long as necessary to write.
    // //  Note that the file lock management becomes a little tricky.

    // // What about OS pipes?  the message passing is very simple — no risk of
    // // deadlocks as long as the message-passing is one-way
    

    // ctx.ProcessCargo = Some(start_cargo_process(macro_coordination_tcp_port));


    // let thread_join_handle = thread::spawn(move || {
    //     // some work here
    // });

    // let (tx, rx) = mpsc::channel();

    // let tx1 = tx.clone();
    // thread::spawn(move || {
    //     let vals = vec![
    //         String::from("hi"),
    //         String::from("from"),
    //         String::from("the"),
    //         String::from("thread"),
    //     ];

    //     for val in vals {
    //         tx1.send(val).unwrap();
    //         thread::sleep(Duration::from_secs(1));
    //     }
    // });

    // thread::spawn(move || {
    //     let vals = vec![
    //         String::from("more"),
    //         String::from("messages"),
    //         String::from("for"),
    //         String::from("you"),
    //     ];

    //     for val in vals {
    //         tx.send(val).unwrap();
    //         thread::sleep(Duration::from_secs(1));
    //     }
    // });

    // for received in rx {
    //     println!("Got: {}", received);
    // }



    // //TODO: start macro coordination server
    // ctx.MacroCoordinationThread = Some(macro_coordination::start_server());
    // /*
    // Option A: dump to an append-only file; load that file after compilation
    // Option B: open a simple HTTP server 
    // */


    // ctx.MacroCoordinationThread.unwrap().attach_listener("finish", |data| {
    //     //use the dumped data gathered by macros:
    //     // - location of all pax files to create a work queue for parsing
    //     // - paths to import for PropertiesCoproduct members, to code-gen PropertiesCoproduct and its Cargo.toml
    // });


    // //TODO: start cargo build
    // // ctx.CargoBuildThread = Some(start_cargo_build_thread())    


    // //Await completion of both threads
    
    // //TODO: perform any necessary codegen, incl. patched Cargo.toml, into temp dir
    // //TODO: run cargo build again; generate .wasm (or other platform-native lib)
    // //TODO: start websocket server
    // //TODO: start demo harness, load cartridge
    // //TODO: establish duplex connection to WS server from cartridge
    // //TODO: start parsing Pax files

    Ok(())
}



fn start_cargo_process(macro_coordination_tcp_port: u16) -> () {
    
    // let process = match Command::new("wc")
    //                             .stdin(Stdio::piped())
    //                             .stdout(Stdio::piped())
    //                             .spawn() {
    //     Err(why) => panic!("couldn't spawn wc: {}", why),
    //     Ok(process) => process,
    // };

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

