use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use color_eyre::eyre::{eyre, Report, Result};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn command() -> App<'static, 'static> {
    SubCommand::with_name("docs")
        .about("Browse Pax documentation from the CLI")
        .subcommand(SubCommand::with_name("list").about("List available docs"))
        .subcommand(
            SubCommand::with_name("open")
                .about("Open a doc page in a pager")
                .arg(
                    Arg::with_name("doc")
                        .help("Doc slug, path, or title")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("search")
                .about("Search docs by full text")
                .arg(
                    Arg::with_name("query")
                        .help("Search query. Use | for OR; quote it in your shell")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("limit")
                        .long("limit")
                        .takes_value(true)
                        .default_value("20")
                        .help("Maximum number of results to show"),
                )
                .arg(
                    Arg::with_name("open")
                        .long("open")
                        .takes_value(false)
                        .help("Open the top result in a pager"),
                ),
        )
        .subcommand(docs_build_command(
            "build",
            "Build docs assets (API, examples, and search index)",
        ))
        .subcommand(docs_build_command(
            "rebuild",
            "Deprecated alias for `pax-cli docs build`",
        ))
}

fn docs_build_command(name: &'static str, about: &'static str) -> App<'static, 'static> {
    let mut command = SubCommand::with_name(name)
        .about(about)
        .arg(
            Arg::with_name("workspace")
                .long("workspace")
                .takes_value(true)
                .help("Path to the Pax workspace (defaults to current directory)"),
        )
        .arg(
            Arg::with_name("watch")
                .long("watch")
                .takes_value(false)
                .help("Continuously build docs assets when docs or Rust sources change"),
        )
        .arg(
            Arg::with_name("skip-examples")
                .long("skip-examples")
                .takes_value(false)
                .help("Refresh example metadata, but skip building runnable example bundles"),
        );

    if name == "rebuild" {
        command = command.setting(AppSettings::Hidden);
    }

    command
}

pub fn handle(args: &ArgMatches<'_>) -> Result<(), Report> {
    match args.subcommand() {
        ("list", Some(sub_args)) => handle_list(sub_args),
        ("open", Some(sub_args)) => handle_open(sub_args),
        ("search", Some(sub_args)) => handle_search(sub_args),
        ("build", Some(sub_args)) | ("rebuild", Some(sub_args)) => handle_build(sub_args),
        _ => handle_list(args),
    }
}

fn handle_list(_args: &ArgMatches<'_>) -> Result<(), Report> {
    let entries = pax_docs::entries().map_err(|err| eyre!(err.to_string()))?;
    for (idx, entry) in entries.iter().enumerate() {
        let indent = "  ".repeat(entry.depth as usize);
        println!("{}. {}{} | {}", idx + 1, indent, entry.title, entry.summary);
    }

    Ok(())
}

fn handle_open(args: &ArgMatches<'_>) -> Result<(), Report> {
    let doc = args
        .value_of("doc")
        .ok_or_else(|| eyre!("doc name is required"))?;

    let entry = if let Ok(id) = doc.parse::<usize>() {
        let entries = pax_docs::entries().map_err(|err| eyre!(err.to_string()))?;
        if id == 0 || id > entries.len() {
            return Err(eyre!("Doc id {id} is out of range"));
        }
        &entries[id - 1]
    } else {
        pax_docs::find_entry(doc)
            .map_err(|err| eyre!(err.to_string()))?
            .ok_or_else(|| eyre!("No docs entry found for '{doc}'"))?
    };

    let rendered = render_markdown(&entry.body_markdown);
    page_output(&rendered)
}

fn handle_search(args: &ArgMatches<'_>) -> Result<(), Report> {
    let query = args
        .value_of("query")
        .ok_or_else(|| eyre!("query is required"))?;
    let limit = args
        .value_of("limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(20);
    let open_top = args.is_present("open");

    let hits = pax_docs::search(query, limit).map_err(|err| eyre!(err.to_string()))?;

    let entries = pax_docs::entries().map_err(|err| eyre!(err.to_string()))?;
    if hits.is_empty() {
        println!("No matches for '{query}'.");
        return Ok(());
    }

    if open_top {
        let top = &entries[hits[0].entry_index];
        let rendered = render_markdown(&top.body_markdown);
        return page_output(&rendered);
    }

    let top_score = hits.first().map(|hit| hit.score).unwrap_or(0.0);

    for hit in hits.iter() {
        if let Some(entry) = entries.get(hit.entry_index) {
            let pct = if top_score > 0.0 {
                (hit.score / top_score) * 100.0
            } else {
                0.0
            };
            let indent = "  ".repeat(entry.depth as usize);
            println!(
                "{:>5.1}% {}. {}{} | {}",
                pct,
                hit.entry_index + 1,
                indent,
                entry.title,
                entry.summary
            );
        }
    }

    Ok(())
}

fn handle_build(args: &ArgMatches<'_>) -> Result<(), Report> {
    let workspace = if let Some(path) = args.value_of("workspace") {
        PathBuf::from(path)
    } else {
        let cwd = std::env::current_dir()?;
        find_workspace_root(&cwd).unwrap_or(cwd)
    };
    let options = DocsBuildOptions {
        skip_examples: args.is_present("skip-examples"),
    };

    if args.is_present("watch") {
        return handle_build_watch(&workspace, options);
    }

    build_docs(&workspace, options)?;
    println!("Docs built. Re-run pax-cli to pick up the latest docs.");
    Ok(())
}

#[derive(Clone, Copy)]
struct DocsBuildOptions {
    skip_examples: bool,
}

fn handle_build_watch(workspace: &Path, options: DocsBuildOptions) -> Result<(), Report> {
    println!(
        "Watching docs sources under {}. Press Ctrl-C to stop.",
        workspace.display()
    );
    println!("Tip: run `mdbook serve pax-docs/book` in another shell for browser live reload.");
    if options.skip_examples {
        println!("Example bundle builds are disabled for this watch session.");
    }

    if let Err(err) = build_docs(workspace, options) {
        eprintln!("Initial docs build failed: {err}");
    } else {
        println!("Initial docs build complete.");
    }

    let mut previous = watch_snapshot(workspace, options)?;
    loop {
        thread::sleep(Duration::from_secs(1));
        let current = match watch_snapshot(workspace, options) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                eprintln!("Failed to inspect docs sources: {err}");
                continue;
            }
        };
        if current == previous {
            continue;
        }

        println!(
            "Docs source changed: {}",
            describe_watch_changes(&previous, &current)
        );

        let mut pending = current;
        loop {
            if let Err(err) = build_docs(workspace, options) {
                eprintln!("Docs build failed: {err}");
            } else {
                println!("Docs build complete.");
            }

            let after_rebuild = match watch_snapshot(workspace, options) {
                Ok(snapshot) => snapshot,
                Err(err) => {
                    eprintln!("Failed to inspect docs sources after build: {err}");
                    break;
                }
            };
            if after_rebuild == pending {
                previous = after_rebuild;
                break;
            }

            println!("Docs source changed during build; building again.");
            pending = after_rebuild;
        }
    }
}

fn build_docs(workspace: &Path, options: DocsBuildOptions) -> Result<(), Report> {
    let highlight_script = workspace
        .join("pax-docs")
        .join("scripts")
        .join("generate_pax_highlight.py");
    let highlight_script_arg = highlight_script.to_string_lossy().to_string();
    run_command(workspace, "python3", &[highlight_script_arg.as_str()], &[])?;

    run_cargo_command(
        workspace,
        &["run", "-p", "pax-docs", "--bin", "gen_api_docs"],
        &[],
    )?;

    let current_exe = std::env::current_exe()?;
    let current_exe = current_exe.to_string_lossy().to_string();
    let workspace_arg = workspace.to_string_lossy().to_string();
    let mut example_args = vec![
        "run",
        "-p",
        "pax-docs",
        "--bin",
        "gen_example_docs",
        "--",
        "--workspace",
        workspace_arg.as_str(),
    ];
    if options.skip_examples {
        example_args.push("--skip-build");
    }
    run_cargo_command(
        workspace,
        &example_args,
        &[("PAX_DOCS_PAX_CLI", current_exe.as_str())],
    )?;

    let force_token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string();
    run_cargo_command(
        workspace,
        &["build", "-p", "pax-docs"],
        &[("PAX_DOCS_FORCE_REBUILD", force_token.as_str())],
    )?;

    Ok(())
}

#[derive(Clone, PartialEq, Eq)]
struct WatchFingerprint {
    len: u64,
    modified_nanos: u128,
}

fn watch_snapshot(
    workspace: &Path,
    options: DocsBuildOptions,
) -> Result<BTreeMap<PathBuf, WatchFingerprint>, Report> {
    let mut snapshot = BTreeMap::new();
    collect_watch_snapshot(workspace, workspace, options, &mut snapshot)?;
    Ok(snapshot)
}

fn collect_watch_snapshot(
    workspace: &Path,
    dir: &Path,
    options: DocsBuildOptions,
    snapshot: &mut BTreeMap<PathBuf, WatchFingerprint>,
) -> Result<(), Report> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if should_skip_watch_dir(workspace, &path, options) {
                continue;
            }
            collect_watch_snapshot(workspace, &path, options, snapshot)?;
            continue;
        }

        if !file_type.is_file() || !should_watch_file(workspace, &path, options) {
            continue;
        }

        let metadata = entry.metadata()?;
        let modified_nanos = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let relative_path = path.strip_prefix(workspace).unwrap_or(&path).to_path_buf();
        snapshot.insert(
            relative_path,
            WatchFingerprint {
                len: metadata.len(),
                modified_nanos,
            },
        );
    }

    Ok(())
}

fn should_skip_watch_dir(workspace: &Path, path: &Path, options: DocsBuildOptions) -> bool {
    let name = match path.file_name().and_then(|name| name.to_str()) {
        Some(name) => name,
        None => return false,
    };
    if matches!(
        name,
        ".git" | ".hg" | ".svn" | ".pax" | "node_modules" | "target"
    ) {
        return true;
    }

    if path == workspace.join("pax-docs").join("book").join("book")
        || path
            == workspace
                .join("pax-docs")
                .join("book")
                .join("src")
                .join("_pax_examples")
    {
        return true;
    }

    options.skip_examples && path == workspace.join("examples").join("src")
}

fn should_watch_file(workspace: &Path, path: &Path, options: DocsBuildOptions) -> bool {
    if path.starts_with(workspace.join("examples").join("src")) {
        return !options.skip_examples;
    }

    if path
        == workspace
            .join("pax-docs")
            .join("book")
            .join("src")
            .join("versions.json")
    {
        return true;
    }

    let extension = path.extension().and_then(|extension| extension.to_str());
    if extension == Some("rs") {
        return true;
    }

    if path.starts_with(
        workspace
            .join("pax-docs")
            .join("book")
            .join("src")
            .join("api"),
    ) {
        let api_file = path.file_name().and_then(|name| name.to_str());
        return matches!(api_file, Some("crates.txt" | "public_crates.txt"));
    }

    matches!(
        extension,
        Some("css" | "js" | "json" | "md" | "pest" | "py" | "toml")
    )
}

fn describe_watch_changes(
    previous: &BTreeMap<PathBuf, WatchFingerprint>,
    current: &BTreeMap<PathBuf, WatchFingerprint>,
) -> String {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();

    for path in current.keys() {
        if !previous.contains_key(path) {
            added.push(path);
        } else if previous.get(path) != current.get(path) {
            modified.push(path);
        }
    }

    for path in previous.keys() {
        if !current.contains_key(path) {
            removed.push(path);
        }
    }

    let mut parts = Vec::new();
    if !added.is_empty() {
        parts.push(format!("{} added", added.len()));
    }
    if !modified.is_empty() {
        parts.push(format!("{} modified", modified.len()));
    }
    if !removed.is_empty() {
        parts.push(format!("{} removed", removed.len()));
    }

    let mut examples = added
        .iter()
        .chain(modified.iter())
        .chain(removed.iter())
        .take(3)
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    if examples.is_empty() {
        return parts.join(", ");
    }
    if added.len() + modified.len() + removed.len() > examples.len() {
        examples.push("...".to_string());
    }
    format!("{} ({})", parts.join(", "), examples.join(", "))
}

fn run_cargo_command(
    workspace: &Path,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<(), Report> {
    run_command(workspace, "cargo", args, env_vars)
}

fn run_command(
    workspace: &Path,
    program: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<(), Report> {
    let mut cmd = Command::new(program);
    cmd.args(args).current_dir(workspace);
    for (key, value) in env_vars {
        cmd.env(key, value);
    }
    let status = cmd
        .status()
        .map_err(|err| eyre!("Failed to run {program} {}: {err}", args.join(" ")))?;
    if !status.success() {
        return Err(eyre!("Command failed: {program} {}", args.join(" ")));
    }
    Ok(())
}

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join("Cargo.toml");
        if candidate.exists() {
            if let Ok(contents) = fs::read_to_string(&candidate) {
                if contents.contains("[workspace]") {
                    return Some(current);
                }
            }
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn render_markdown(markdown: &str) -> String {
    const RESET: &str = "\u{1b}[0m";
    const BOLD: &str = "\u{1b}[1m";
    const UNDERLINE: &str = "\u{1b}[4m";
    const CODE: &str = "\u{1b}[96m";

    let mut out = String::new();
    let mut in_code_block = false;

    for line in markdown.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("<!--") && trimmed.ends_with("-->") {
            continue;
        }

        if let Some(signature) = parse_api_signature_html(trimmed) {
            out.push_str(CODE);
            out.push_str(&signature);
            out.push_str(RESET);
            out.push('\n');
            continue;
        }

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            out.push_str(CODE);
            out.push_str(line);
            out.push_str(RESET);
            out.push('\n');
            continue;
        }

        if let Some((level, title)) = parse_heading(trimmed) {
            if level == 1 {
                out.push_str(BOLD);
                out.push_str(UNDERLINE);
                out.push_str(title);
                out.push_str(RESET);
                out.push('\n');
                continue;
            }

            out.push_str(BOLD);
            out.push_str(title);
            out.push_str(RESET);
            out.push('\n');
            continue;
        }

        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }

        out.push_str(&render_inline(line, CODE, RESET, BOLD));
        out.push('\n');
    }

    out
}

fn parse_api_signature_html(line: &str) -> Option<String> {
    let suffix = "</code></pre>";
    let line = line.strip_prefix("<pre><code class=\"")?;
    let (classes, inner) = line.split_once("\">")?;
    if !classes
        .split_whitespace()
        .any(|class| class == "api-signature")
    {
        return None;
    }
    let inner = inner.strip_suffix(suffix)?;
    Some(decode_html_entities(&strip_html_tags(inner)))
}

fn strip_html_tags(text: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn decode_html_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn parse_heading(line: &str) -> Option<(usize, &str)> {
    let mut chars = line.chars();
    let mut level = 0usize;
    while let Some('#') = chars.clone().next() {
        level += 1;
        chars.next();
    }
    if level == 0 {
        return None;
    }
    let title = line[level..].trim();
    if title.is_empty() {
        None
    } else {
        Some((level, title))
    }
}

fn render_inline(line: &str, code: &str, reset: &str, bold: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    let mut in_bold = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '`' {
            in_code = !in_code;
            out.push_str(if in_code { code } else { reset });
            continue;
        }

        if ch == '*' && chars.peek() == Some(&'*') {
            chars.next();
            in_bold = !in_bold;
            out.push_str(if in_bold { bold } else { reset });
            continue;
        }

        out.push(ch);
    }

    if in_code || in_bold {
        out.push_str(reset);
    }

    out
}

fn page_output(text: &str) -> Result<(), Report> {
    match spawn_pager() {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            }
            let status = child.wait()?;
            if !status.success() {
                println!("{text}");
            }
            Ok(())
        }
        Err(_) => {
            println!("{text}");
            Ok(())
        }
    }
}

fn spawn_pager() -> std::io::Result<Child> {
    if let Ok(pager) = std::env::var("PAGER") {
        return spawn_shell_or_direct(&pager);
    }

    Command::new("less").arg("-R").stdin(Stdio::piped()).spawn()
}

fn spawn_shell_or_direct(command: &str) -> std::io::Result<Child> {
    if command.trim().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "PAGER is empty",
        ));
    }

    if command.contains(' ') {
        if cfg!(windows) {
            Command::new("cmd")
                .arg("/C")
                .arg(command)
                .stdin(Stdio::piped())
                .spawn()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .stdin(Stdio::piped())
                .spawn()
        }
    } else {
        Command::new(command).stdin(Stdio::piped()).spawn()
    }
}
