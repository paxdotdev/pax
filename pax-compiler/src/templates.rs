//Use Tera to manage templates

use walkdir::WalkDir;
use std::fs;
use fs::create_dir_all;

use std::path::Path;

use std::env::temp_dir;


use include_dir::{include_dir, Dir};
use tera::{Context, Tera};


static ROOT_PATH : &str = "$CARGO_MANIFEST_DIR/templates";
static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

// Given the provided `template_sub_dir` (relative to @/templates), build the template directory
// and return a String path to the temp directory on disk where the template is built
pub fn compile_and_mount_template_directory(template_sub_dir: &str) -> String {



    //0. allocate a new temp directory
    let new_uuid = uuid::Uuid::new_v4().to_string();
    //As an alternative to `temp_dir`, consider a `.pax` folder at the root of
    //the targeted project.  In particular, that offers a single point of cleanup and
    //a greater assurance of control over the continuity of the files at the path
    let mount_path = Path::new(&temp_dir()).join("pax-compiler").join(new_uuid);
    create_dir_all(&mount_path).unwrap();

    println!("new mount path {}", &mount_path.to_str().unwrap());

    //1. enumerate all files recursively at `template_path`
    for file in TEMPLATE_DIR.find("parser-bin-harness/**/*").unwrap() {

        let root_path = Path::new(ROOT_PATH);
        let relative_path = file.path().to_str().unwrap().replace(root_path.to_str().unwrap(), "");


        let x = Context::new();
        // x.insert()

        // Tera::one_off(file.as_file().unwrap().contents_utf8().unwrap(), )

        println!("File: {:?}", file);
    }
    // for e in WalkDir::new(template_path).into_iter().filter_map(|e| e.ok()) {
    //     if e.metadata().unwrap().is_file() {
    //         println!("{}", e.path().display());
    //     }
    // }

    //2. load each file, process with Tera, write to temp directory

    //When finished, should have a `hydrated` copy of the template directory
    //available for manipulation at the specified place on disk

    todo!()
}