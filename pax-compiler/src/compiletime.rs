#[macro_use]
extern crate pest_derive;
/// logic used e.g. within macros during the parsing/compilation processes

use std::{env, fs};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use uuid::Uuid;
use pax_message::ComponentDefinition;

mod parser;

pub fn get_uuid() -> String {
    Uuid::new_v4().to_string()
}



pub struct ManifestContext {
    /// Used to track which files/sources have been visited during parsing,
    /// to prevent duplicate parsing
    pub visited_source_ids: HashSet<String>,
    pub component_definitions: Vec<ComponentDefinition>,

}



pub fn process_pax_file_for_pascal_identifiers(file_path: &str) -> Vec<String> {

    let mut dir_path = PathBuf::from(file_path);
    dir_path.pop();

    let mut lib_path = PathBuf::from(dir_path);
    lib_path.push("lib.rs");

    //Look for a same-named .pax file, e.g. lib.pax alongside lib.rs
    env::set_current_dir(file_path);
    let mut absolute_lib_path = fs::canonicalize(&lib_path).unwrap();
    absolute_lib_path.set_extension("pax");

    println!("Probing for file: {:?}", absolute_lib_path);

    let pax = fs::read_to_string(absolute_lib_path).unwrap(); //TODO: def. need error handling
    println!("found pax (macro generation phase) -  {}", pax);
    let symbols = parser::parse_file_for_symbols_in_template(&pax);
    println!("{:?}", symbols);
    symbols
}

pub fn process_pax_file_for_component_definition(symbol_name: &str, file_path: &str, module_path: &str) -> ComponentDefinition {

    let mut dir_path = PathBuf::from(file_path);
    dir_path.pop();

    let mut lib_path = PathBuf::from(dir_path);
    lib_path.push("lib.rs");

    //Look for a same-named .pax file, e.g. lib.pax alongside lib.rs
    env::set_current_dir(file_path);
    let mut absolute_lib_path = fs::canonicalize(&lib_path).unwrap();
    absolute_lib_path.set_extension("pax");


    //TODO: check if this .pax file exists; load its contents as a string; pass to parser
    println!("Probing for file: {:?}", absolute_lib_path);

    //TODO: since we're already parsing .pax on this pass... we might as well generate full RIL.
    //      No more pax parsing would be needed on second compilation pass; however:
    //      if we use RIL as the "data dump" format to hand off the parsed data between processes,
    //      then we'll ALSO need to be able to parse RIL back into Definition structs... not worth it!
    //      So: we can parse all of .pax here, gather into Definition objects, pass back to main process via TCP,
    //          and call it a day
    let pax = fs::read_to_string(absolute_lib_path).unwrap(); //TODO: def. need error handling
    println!("found pax: {}", pax);

    parser::parse_component_from_pax_file(pax.as_str(), symbol_name, false);

    println!("parsing successful");
    unimplemented!()
}


//
//
//
// pub fn process_file(file_path: &str, module_path: &str) {
//     //Look for a same-named .pax file, e.g. lib.pax alongside lib.rs
//     env::set_current_dir(file_path);
//     let mut absolute_file_path = fs::canonicalize(file_path).unwrap();
//
//     absolute_file_path.set_extension("pax");
//
//     println!("Probing for file: {:?}", absolute_file_path);
//
//     let pax = fs::read_to_string(absolute_file_path).unwrap();
//     println!("found pax: {}", pax);
//
//
// }