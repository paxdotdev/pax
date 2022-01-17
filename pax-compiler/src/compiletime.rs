/// logic used e.g. within macros during the parsing/compilation processes

use std::{env, fs};
use std::path::PathBuf;

pub fn process_file(path: &str) {
    //Look for a same-named .pax file, e.g. lib.pax alongside lib.rs
    env::set_current_dir(path);
    let mut absolute_current_rust_file_path = fs::canonicalize(path).unwrap();

    absolute_current_rust_file_path.set_extension("pax");

    //TODO: check if this .pax file exists; load its contents as a string; pass to parser
    println!("Probing for file: {:?}", absolute_current_rust_file_path);

    //TODO: since we're already parsing .pax on this pass... we might as well generate full RIL.
    //      No more pax parsing would be needed on second compilation pass; however:
    //      if we use RIL as the "data dump" format to hand off the parsed data between processes,
    //      then we'll ALSO need to be able to parse RIL back into Definition structs... not necessary!
    //
    //      So: we can parse all of .pax here, gather into Definition objects, pass back to main process,
    //          and call it a day
    


}