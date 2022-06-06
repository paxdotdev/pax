#[macro_use]
extern crate pest_derive;

use tokio::net::{TcpListener, TcpStream};

use tokio::task::yield_now;
use tokio::{select, task};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{Sender, Receiver, UnboundedReceiver};
use tokio_stream::wrappers::{ReceiverStream};

mod api;
mod server;

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
// use pax_compiler_api::PaxManifest;

use toml_edit::{Document, value};
use uuid::Uuid;

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

struct RunContext {
    target: String,
    path: String,
    handle: Handle,
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

