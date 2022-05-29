#[macro_use]
extern crate pest_derive;

use tokio::net::{TcpListener, TcpStream};

use tokio::task::yield_now;
use tokio::{select, task};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{Sender, Receiver, UnboundedReceiver};
use tokio_stream::wrappers::{ReceiverStream};

mod api;
mod templates;
mod server;

use templates::*;

use std::io::{Error};
use std::task::{Poll, Context};
use std::{fs, thread::{Thread, self}, time::Duration};
use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use clap::{App, AppSettings, Arg};

use futures::prelude::*;
use include_dir::{Dir, include_dir};

// use crate::parser::message::*;
use serde_json::Value;
use tera::Tera;
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio_serde::SymmetricallyFramed;
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};
use tokio_serde::formats::*;
use pax_compiler_api::PaxManifest;



use toml_edit::{Document, value};
use uuid::Uuid;


fn tryout() {
    let toml = r#"
        "hello" = 'toml!' # comment
        ['a'.b]
    "#;
    let mut doc = toml.parse::<Document>().expect("invalid doc");

    assert_eq!(doc.to_string(), toml);
    // let's add a new key/value pair inside a.b: c = {d = "hello"}
    doc["a"]["b"]["c"]["d"] = value("hello");
    // autoformat inline table a.b.c: { d = "hello" }
    doc["a"]["b"]["c"].as_inline_table_mut().map(|t| t.fmt());
    let expected = r#"
        "hello" = 'toml!' # comment
        ['a'.b]
        c = { d = "hello" }
    "#;
    assert_eq!(doc.to_string(), expected);
}


#[tokio::main]
async fn main() -> Result<(), Error> {
    let matches = App::new("pax")
        .name("pax")
        .bin_name("pax")
        .about("Pax language compiler and dev tooling")
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
                target,
                path,
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

async fn run_macro_coordination_server(mut red_phone: UnboundedReceiver<bool>, return_data_channel : tokio::sync::oneshot::Sender<PaxManifest>, macro_coordination_tcp_port: u16) -> Result<(), Error> {

    let listener = TcpListener::bind(format!("127.0.0.1:{}",macro_coordination_tcp_port)).await.unwrap();

    loop {
        // tokio::select! {
        //     _ = red_phone.recv() => {
        //         //for now, any message from parent is the shutdown message
        //         println!("Red phone message received; shutting down thread");
        //         break;
        //     }
        //     _ = listener.accept() => {
        //         println!("TCP message received");
        //
        //         let tcp_msg = "TODO".as_bytes();
        //
        //         return_data_channel.send(PaxManifest::deserialize(tcp_msg));
        //     }
        // }


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

struct RunHelpers {}
impl RunHelpers {

//     pub fn create_parser_cargo_file(working_dir: &str, output_dir: &PathBuf) -> PathBuf {
//         //Load existing Cargo.toml
//         //Parse with toml parser -- pull `features` and `dependencies` into `original_features` and `original_dependencies`.  Serialize the rest into `original_contents_cleaned`.
//
//         let existing_cargo_contents = fs::read_to_string(
//             Path::new(&working_dir)
//                 .join("Cargo.toml")
//         ).expect(&("Couldn't find Cargo.toml in specified directory: ".to_string() + working_dir));
//
//         let mut parsed_existing_cargo = existing_cargo_contents.parse::<Document>().expect("invalid TOML document -- verify the Cargo.toml in the specified working directory");
//
//         let mut original_dependencies = &parsed_existing_cargo["dependencies"];
//
//         //Remove any existing entries that we're going to add, to ensure no duplicates
//         match original_dependencies.to_owned().into_table() {
//             Ok(mut original_dependencies_table) => {
//                 //These entries must exist in the parser-cargo `template`, too
//                 original_dependencies_table.remove("lazy_static");
//                 original_dependencies_table.remove("pax-compiler");
//             },
//             _ => {}
//         }
//
//         let original_dependencies = original_dependencies.to_string().clone();
//         let original_features = match parsed_existing_cargo.get("features") {
//             Some(feats) => {feats.to_string()},
//             _ => {"".to_string()}
//         };
//         parsed_existing_cargo.remove("dependencies");
//         parsed_existing_cargo.remove("features");
//         let original_contents_cleaned = parsed_existing_cargo.to_string();
//
//         //Populate template data structure; compile template
//         let cpa = TemplateArgsParserCargo {
//             original_contents_cleaned,
//             original_features,
//             original_dependencies,
//         };
//
//         let template_parser_cargo = TEMPLATE_DIR.get_file("parser-cargo/Cargo.toml").unwrap().contents_utf8().unwrap();
//
//         let cargo_file_contents = Tera::one_off(&template_parser_cargo, &tera::Context::from_serialize(cpa).unwrap(), false).unwrap();
//
//         let mut ret_path = get_or_create_pax_tmp_directory(working_dir)
//             .join(Uuid::new_v4().to_string());
//
//         fs::create_dir_all(&ret_path);
//
//         ret_path = ret_path.join("Cargo.toml");
//
//         fs::write(&ret_path, cargo_file_contents);
//         ret_path
//         //Generate templated Cargo.toml into output_dir; return full path to that file
//
//     }
}
fn get_or_create_pax_directory(working_dir: &str) -> PathBuf {
    let mut working_path = std::path::Path::new(working_dir).join(".pax");
    std::fs::create_dir_all( &working_path);
    working_path
}
const TMP_DIRECTORY_NAME: &str = "tmp";
fn get_or_create_pax_tmp_directory(working_dir: &str) -> PathBuf {
    let tmp = Path::new(&get_or_create_pax_directory(working_dir)).join(TMP_DIRECTORY_NAME);
    std::fs::create_dir_all( &tmp);
    tmp
}

static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");












/// For the specified file path or current working directory, first compile Pax project,
/// then run it with a patched build of the `chassis` appropriate for the specified platform
async fn perform_run(ctx: RunContext) -> Result<(), Error> {

    println!("Performing run");

    let tmp_dir =  get_or_create_pax_tmp_directory(&ctx.path);

    //TODO: handle stand-alone .pax files

    //TODO: automatically inject missing deps into host Cargo.toml (or offer to do so)
    //      alternatively — offer a separate command, `pax init .` for example, which
    //      can generate empty projects or patch existing ones.  in this world,
    //      we can handle errors in running `cargo .. --features parser` and prompt
    //      user to run `pax init`
    // let parser_cargo_file_path = RunHelpers::create_parser_cargo_file(&ctx.path, &tmp_dir);

    // Run parser bin from host project with `--features parser`
    let cargo_run_parser_future = Command::new("cargo")
        .current_dir(&ctx.path)
        .arg("run")
        .arg("--features")
        .arg("parser")
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute parser binary");

    let output = cargo_run_parser_future
        .wait_with_output()
        .await
        .unwrap();

    let out = String::from_utf8(output.stdout).unwrap();
    println!("{}", &out);

    assert!(output.status.success());



    //4. Run compiled `parser binary` from tmp, which reports back to parser coordination server
    //5. After PaxManifest is received by main thread, shut down parser coordination server
    //6. Codegen:
    //   - Properties Coproduct
    //   - Cartridge
    //   - Cargo.toml for the appropriate `chassis` (including patches for Properties Coproduct & Cartridge)
    //7. Build the appropriate `chassis` from source, with the patched `Cargo.toml`, Properties Coproduct, and Cartridge from above
    //8. Run dev harness, with freshly built chassis plugged in

    //see pax-compiler-sequence-diagram.png



    /*
    Problem: the location of the cargo file acts as the root for relative paths, e.g. `../pax-lang`
    Possible solutions:
        - require manual or one-time addition/injection of the [[bin]] target, plus the `pax-compiler` dependency and the `parser = ["pax-std/parser"]` feature
        - gen the cargo file into PWD _as_ Cargo.toml; restore the old cargo file afterwards (store as Cargo.toml.bak, perhaps)
        - gen a complete copy of the project elsewhere (still would have trouble with ../ paths)
        - try to patch any "../" paths detected in the input Cargo.toml with `fs::canonicalize`d full paths
            ^ this feels slightly hacky... but also maybe the cleanest option here
              Note: tried it by hand (expanding absolute paths) and it worked a charm

              Maybe just regex replace any `../` for now?  Could make more robust for e.g. Windows


    ------

    zack@Quixote pax-example % cargo run --features parser --manifest-path ./.pax/tmp/8ebadfe9-61ce-4a27-bdf7-ab6b0b2666af/Cargo.toml
    error: failed to get `pax-lang` as a dependency of package `pax-example v0.0.1 (/Users/zack/code/pax-lang/pax-example/.pax/tmp/8ebadfe9-61ce-4a27-bdf7-ab6b0b2666af)`

    Caused by:
      failed to load source for dependency `pax-lang`

    Caused by:
      Unable to update /Users/zack/code/pax-lang/pax-example/.pax/tmp/pax-lang

    Caused by:
      failed to read `/Users/zack/code/pax-lang/pax-example/.pax/tmp/pax-lang/Cargo.toml`

    Caused by:
      No such file or directory (os error 2)

     */



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

