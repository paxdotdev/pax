use std::fs;
use std::path::{Path, PathBuf};

use clap::{App, Arg, ArgMatches, SubCommand};
use color_eyre::eyre::{eyre, Report, Result};
use pax_compiler::svg_import::{component_name_from_path, format_number, import_svg_file};

pub fn command() -> App<'static, 'static> {
    SubCommand::with_name("svg-import")
        .about("Validate or eject an SVG as editable Pax path source")
        .arg(
            Arg::with_name("input")
                .help("SVG file to import")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("component")
                .long("component")
                .takes_value(true)
                .help("PascalCase component name for ejected Rust"),
        )
        .arg(
            Arg::with_name("stdout")
                .long("stdout")
                .takes_value(false)
                .help("Print generated Pax template source to stdout"),
        )
        .arg(
            Arg::with_name("out")
                .long("out")
                .takes_value(true)
                .help("Eject editable <path>.rs and <path>.pax component source"),
        )
        .arg(
            Arg::with_name("force")
                .long("force")
                .takes_value(false)
                .help("Overwrite existing files when used with --out"),
        )
}

pub fn handle(args: &ArgMatches<'_>) -> Result<(), Report> {
    let input = PathBuf::from(args.value_of("input").expect("input is required"));
    let component = args
        .value_of("component")
        .map(str::to_string)
        .unwrap_or_else(|| component_name_from_path(&input));
    let import = import_svg_file(&input)?;

    if args.is_present("stdout") {
        print!("{}", import.pax_source);
    }

    if let Some(out) = args.value_of("out") {
        let written = eject_component(
            Path::new(out),
            &component,
            &import.pax_source,
            args.is_present("force"),
        )?;
        println!("wrote {}", written.rs_path.display());
        println!("wrote {}", written.pax_path.display());
    }

    if !args.is_present("stdout") && args.value_of("out").is_none() {
        println!(
            "SVG import ok: {} SVG path(s), {} polygon(s), {} generated Pax Path node(s)",
            import.svg_path_count, import.svg_polygon_count, import.generated_path_count
        );
        println!(
            "viewBox: {} {} {} {}",
            format_number(import.view_box.min_x),
            format_number(import.view_box.min_y),
            format_number(import.view_box.width),
            format_number(import.view_box.height)
        );
        println!("source_sha256: {}", import.source_sha256);
        if !import.warnings.is_empty() {
            println!("warnings:");
            for warning in &import.warnings {
                println!("  - {warning}");
            }
        }
    }

    Ok(())
}

struct EjectedFiles {
    rs_path: PathBuf,
    pax_path: PathBuf,
}

fn eject_component(
    out_prefix: &Path,
    component: &str,
    pax_source: &str,
    force: bool,
) -> Result<EjectedFiles, Report> {
    let rs_path = out_prefix.with_extension("rs");
    let pax_path = out_prefix.with_extension("pax");
    if !force {
        for path in [&rs_path, &pax_path] {
            if path.exists() {
                return Err(eyre!(
                    "{} already exists; pass --force to overwrite",
                    path.display()
                ));
            }
        }
    }
    if let Some(parent) = rs_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = pax_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let pax_file_name = pax_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| eyre!("--out must resolve to a valid .pax file name"))?;
    let rust_source = format!(
        "use pax_kit::*;\n\n\
         #[pax]\n\
         #[custom(Default)]\n\
         #[file(\"{pax_file_name}\")]\n\
         pub struct {component} {{\n\
         \x20   pub draw_start: Property<UnitValue>,\n\
         \x20   pub draw_end: Property<UnitValue>,\n\
         }}\n\n\
         impl Default for {component} {{\n\
         \x20   fn default() -> Self {{\n\
         \x20       Self {{\n\
         \x20           draw_start: Property::new(UnitValue::Unitless(Numeric::F64(0.0))),\n\
         \x20           draw_end: Property::new(UnitValue::Unitless(Numeric::F64(1.0))),\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n"
    );
    fs::write(&rs_path, rust_source)?;
    fs::write(&pax_path, pax_source)?;
    Ok(EjectedFiles { rs_path, pax_path })
}
