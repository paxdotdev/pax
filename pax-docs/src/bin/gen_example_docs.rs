use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

#[path = "../examples.rs"]
#[allow(dead_code)]
mod examples;

#[derive(Default)]
struct Args {
    workspace: Option<PathBuf>,
    skip_build: bool,
}

#[derive(Clone)]
struct ExampleBuildPlan {
    path: String,
    title: String,
    height: Option<u32>,
    files: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = args.workspace.unwrap_or_else(|| {
        manifest_dir
            .parent()
            .expect("pax-docs must live one level below workspace root")
            .to_path_buf()
    });
    let book_src = manifest_dir.join("book").join("src");
    let out_dir = book_src.join(examples::EXAMPLES_ASSET_DIR);

    fs::create_dir_all(&out_dir)?;
    write_gitignore(&out_dir)?;

    let plans = discover_build_plans(&workspace, &book_src)?;
    prune_unreferenced_examples(&out_dir, plans.keys())?;

    if plans.is_empty() {
        println!("No pax-example embeds found.");
        return Ok(());
    }

    let pax_cli = resolve_pax_cli(&workspace);
    for plan in plans.values() {
        process_example(&workspace, &out_dir, &pax_cli, plan, args.skip_build)?;
    }

    Ok(())
}

fn parse_args() -> Result<Args, Box<dyn std::error::Error>> {
    let mut args = Args::default();
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--skip-build" => args.skip_build = true,
            "--workspace" => {
                let Some(path) = iter.next() else {
                    return Err("--workspace requires a path".into());
                };
                args.workspace = Some(PathBuf::from(path));
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }

    Ok(args)
}

fn print_help() {
    println!("gen_example_docs");
    println!();
    println!("USAGE:");
    println!("    gen_example_docs [--workspace <path>] [--skip-build]");
}

fn discover_build_plans(
    workspace: &Path,
    book_src: &Path,
) -> Result<BTreeMap<String, ExampleBuildPlan>, Box<dyn std::error::Error>> {
    let mut plans = BTreeMap::new();
    visit_markdown(book_src, &mut |path, markdown| {
        for embed in examples::discover_embeds(markdown) {
            let Some(example_dir) = examples::example_dir(workspace, &embed.path) else {
                eprintln!("Skipping invalid pax-example path in {}.", path.display());
                continue;
            };
            if !example_dir.join("Cargo.toml").exists() {
                eprintln!(
                    "Skipping pax-example `{}` in {}: missing Cargo.toml.",
                    embed.path,
                    path.display()
                );
                continue;
            }

            let plan = plans
                .entry(embed.path.clone())
                .or_insert_with(|| ExampleBuildPlan {
                    path: embed.path.clone(),
                    title: embed
                        .title
                        .clone()
                        .unwrap_or_else(|| examples::humanize_example_title(&embed.path)),
                    height: embed.height,
                    files: Vec::new(),
                });
            if plan.height.is_none() {
                plan.height = embed.height;
            }
            for file in embed.files {
                if !plan.files.contains(&file) {
                    plan.files.push(file);
                }
            }
        }
    })?;

    for plan in plans.values_mut() {
        if plan.files.is_empty() {
            let Some(example_dir) = examples::example_dir(workspace, &plan.path) else {
                continue;
            };
            plan.files = examples::source_file_relative_paths(&example_dir, &[])
                .into_iter()
                .map(|path| path.to_string_lossy().replace('\\', "/"))
                .collect();
        }
    }

    Ok(plans)
}

fn visit_markdown<F>(dir: &Path, visitor: &mut F) -> io::Result<()>
where
    F: FnMut(&Path, &str),
{
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some(examples::EXAMPLES_ASSET_DIR)
            {
                continue;
            }
            visit_markdown(&path, visitor)?;
            continue;
        }

        if !file_type.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let markdown = fs::read_to_string(&path)?;
        visitor(&path, &markdown);
    }

    Ok(())
}

fn prune_unreferenced_examples<'a>(
    out_dir: &Path,
    referenced: impl Iterator<Item = &'a String>,
) -> io::Result<()> {
    let referenced = referenced.cloned().collect::<BTreeSet<_>>();
    for entry in fs::read_dir(out_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !referenced.contains(name) {
            fs::remove_dir_all(path)?;
        }
    }
    Ok(())
}

fn resolve_pax_cli(workspace: &Path) -> PathBuf {
    if let Ok(path) = std::env::var("PAX_DOCS_PAX_CLI") {
        let path = PathBuf::from(path);
        if path.exists() {
            return path;
        }
    }

    let debug_cli = workspace
        .join("target")
        .join("debug")
        .join(exe_name("pax-cli"));
    if debug_cli.exists() {
        return debug_cli;
    }

    PathBuf::from("pax-cli")
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn process_example(
    workspace: &Path,
    out_dir: &Path,
    pax_cli: &Path,
    plan: &ExampleBuildPlan,
    skip_build: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let example_dir = examples::example_dir(workspace, &plan.path)
        .ok_or_else(|| format!("invalid example path: {}", plan.path))?;
    let example_out = out_dir.join(&plan.path);
    fs::create_dir_all(&example_out)?;

    let source_fingerprint = examples::fingerprint_dir(&example_dir)?;
    let manifest_path = example_out.join("manifest.json");
    let previous_built_fingerprint = read_manifest_string(&manifest_path, "built_fingerprint");
    let app_index = example_out.join("app").join("index.html");
    let needs_build = previous_built_fingerprint.as_deref() != Some(source_fingerprint.as_str())
        || !app_index.exists();

    let mut build_error = None;
    if !skip_build && needs_build {
        if let Err(err) = build_example(pax_cli, workspace, &plan.path) {
            let message = err.to_string();
            eprintln!("Failed to build docs example `{}`: {message}", plan.path);
            build_error = Some(message);
        }
    }

    let build_src = example_dir
        .join(".pax")
        .join("build")
        .join("debug")
        .join("web");
    if build_src.join("index.html").exists() && (!skip_build || !app_index.exists()) {
        sync_dir(&build_src, &example_out.join("app"))?;
    }

    let app_available = example_out.join("app").join("index.html").exists();
    let built_fingerprint = if !skip_build && app_available {
        Some(source_fingerprint.clone())
    } else if app_available {
        previous_built_fingerprint
    } else {
        None
    };
    write_manifest(
        &example_out,
        plan,
        &source_fingerprint,
        built_fingerprint.as_deref(),
        app_available,
        skip_build,
        build_error.as_deref(),
        &example_dir,
    )?;

    if app_available {
        println!("Prepared docs example `{}`.", plan.path);
    } else if skip_build {
        println!(
            "Prepared docs example metadata `{}` (build skipped).",
            plan.path
        );
    } else if build_error.is_some() {
        println!(
            "Prepared docs example metadata `{}` (example build failed).",
            plan.path
        );
    } else {
        println!(
            "Prepared docs example metadata `{}` (no web build output found).",
            plan.path
        );
    }

    Ok(())
}

fn build_example(pax_cli: &Path, workspace: &Path, example_path: &str) -> io::Result<()> {
    let status = Command::new(pax_cli)
        .current_dir(workspace)
        .arg("build")
        .arg("--path")
        .arg(format!("examples/src/{example_path}"))
        .arg("--target")
        .arg("web")
        .arg("--designer=false")
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("failed to build docs example `{example_path}`"),
        ));
    }

    Ok(())
}

fn read_manifest_string(path: &Path, key: &str) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&contents).ok()?;
    value
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn write_manifest(
    example_out: &Path,
    plan: &ExampleBuildPlan,
    source_fingerprint: &str,
    built_fingerprint: Option<&str>,
    app_available: bool,
    build_skipped: bool,
    build_error: Option<&str>,
    example_dir: &Path,
) -> io::Result<()> {
    let source_files = examples::read_source_files(example_dir, &plan.files)
        .into_iter()
        .map(|file| {
            json!({
                "path": file.path,
                "language": file.language,
                "contents": file.contents,
                "truncated": file.truncated,
            })
        })
        .collect::<Vec<_>>();

    let manifest = json!({
        "path": plan.path,
        "title": plan.title,
        "height": plan.height,
        "run_command": examples::run_command(&plan.path),
        "hosted_url": examples::hosted_example_url(&plan.path),
        "source_fingerprint": source_fingerprint,
        "built_fingerprint": built_fingerprint,
        "app": {
            "available": app_available,
            "index": "app/index.html",
            "build_skipped": build_skipped,
            "build_error": build_error,
        },
        "files": source_files,
    });

    let manifest = serde_json::to_string_pretty(&manifest)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    write_if_changed(&example_out.join("manifest.json"), manifest.as_bytes())
}

fn sync_dir(src: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;

    let mut seen = BTreeSet::new();
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        seen.insert(entry.file_name());
        if entry.file_type()?.is_dir() {
            sync_dir(&src_path, &dest_path)?;
        } else {
            let bytes = fs::read(&src_path)?;
            write_if_changed(&dest_path, &bytes)?;
        }
    }

    for entry in fs::read_dir(dest)? {
        let entry = entry?;
        if !seen.contains(&entry.file_name()) {
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }
    }

    Ok(())
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Ok(existing) = fs::read(path) {
        if existing == bytes {
            return Ok(());
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
}

fn write_gitignore(out_dir: &Path) -> io::Result<()> {
    write_if_changed(&out_dir.join(".gitignore"), b"*\n!.gitignore\n")
}
