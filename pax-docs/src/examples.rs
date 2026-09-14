use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const EXAMPLES_ASSET_DIR: &str = "_pax_examples";
pub const HOSTED_DOCS_BASE_URL: &str = "https://docs.pax.dev";
const MAX_SOURCE_BYTES: usize = 200_000;
const MAX_DEFAULT_SOURCE_FILES: usize = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExampleEmbed {
    pub path: String,
    pub title: Option<String>,
    pub height: Option<u32>,
    pub files: Vec<String>,
    pub inline_source: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExampleSourceFile {
    pub path: String,
    pub language: String,
    pub contents: String,
    pub truncated: bool,
}

#[derive(Clone, Debug)]
struct ParsedExampleEmbed {
    embed: ExampleEmbed,
    start: usize,
    end: usize,
}

pub fn discover_embeds(markdown: &str) -> Vec<ExampleEmbed> {
    parse_embeds(markdown)
        .into_iter()
        .map(|parsed| parsed.embed)
        .collect()
}

pub fn expand_examples_for_cli(markdown: &str, workspace: &Path) -> String {
    let parsed = parse_embeds(markdown);
    if parsed.is_empty() {
        return markdown.to_string();
    }

    let mut out = String::with_capacity(markdown.len());
    let mut cursor = 0usize;
    for parsed_embed in parsed {
        out.push_str(&markdown[cursor..parsed_embed.start]);
        out.push_str(&render_cli_fallback(&parsed_embed.embed, workspace));
        cursor = parsed_embed.end;
    }
    out.push_str(&markdown[cursor..]);
    out
}

pub fn example_dir(workspace: &Path, path: &str) -> Option<PathBuf> {
    normalize_relative_path(path).map(|path| workspace.join("examples").join("src").join(path))
}

pub fn source_file_paths_for_embed(workspace: &Path, embed: &ExampleEmbed) -> Vec<PathBuf> {
    let Some(example_dir) = example_dir(workspace, &embed.path) else {
        return Vec::new();
    };

    source_file_relative_paths(&example_dir, &embed.files)
        .into_iter()
        .map(|path| example_dir.join(path))
        .collect()
}

pub fn discover_example_paths(workspace: &Path) -> io::Result<Vec<String>> {
    let examples_dir = workspace.join("examples").join("src");
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(examples_dir) else {
        return Ok(out);
    };

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path();
        if !path.join("Cargo.toml").exists() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if let Some(path) = normalize_relative_path(name) {
            out.push(path);
        }
    }

    out.sort();
    Ok(out)
}

pub fn read_source_files(example_dir: &Path, requested_files: &[String]) -> Vec<ExampleSourceFile> {
    source_file_relative_paths(example_dir, requested_files)
        .into_iter()
        .filter_map(|path| read_source_file(example_dir, &path))
        .collect()
}

pub fn source_file_relative_paths(example_dir: &Path, requested_files: &[String]) -> Vec<PathBuf> {
    if requested_files.is_empty() {
        return default_source_file_paths(example_dir);
    }

    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for requested in requested_files {
        let Some(path) = normalize_relative_path(requested) else {
            continue;
        };
        if !is_source_file_path(&path) {
            continue;
        }
        if seen.insert(path.clone()) {
            out.push(PathBuf::from(path));
        }
    }
    out
}

pub fn humanize_example_title(path: &str) -> String {
    path.split(['-', '_', '/'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn hosted_example_url(path: &str) -> String {
    format!(
        "{}/{}/{}/app/",
        HOSTED_DOCS_BASE_URL.trim_end_matches('/'),
        EXAMPLES_ASSET_DIR,
        path.trim_matches('/')
    )
}

pub fn run_command(path: &str) -> String {
    format!("pax-cli run --path examples/src/{path} --target web")
}

pub fn render_example_source_markdown(
    workspace: &Path,
    path: &str,
    title: Option<&str>,
) -> Option<String> {
    let example_dir = example_dir(workspace, path)?;
    let source_files = read_source_files(&example_dir, &[]);
    if source_files.is_empty() {
        return None;
    }

    Some(render_source_markdown(
        path,
        title
            .map(|title| title.to_string())
            .unwrap_or_else(|| humanize_example_title(path)),
        &source_files,
    ))
}

fn parse_embeds(markdown: &str) -> Vec<ParsedExampleEmbed> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    let tag = "<pax-example";

    while let Some(relative_start) = markdown[cursor..].find(tag) {
        let start = cursor + relative_start;
        if !has_tag_boundary(markdown, start + tag.len()) {
            cursor = start + tag.len();
            continue;
        }

        let Some(tag_end) = find_tag_end(markdown, start) else {
            break;
        };

        let attrs = &markdown[start + tag.len()..tag_end];
        let Some(embed) = parse_embed_attrs(attrs) else {
            cursor = tag_end + 1;
            continue;
        };

        let mut end = tag_end + 1;
        if !attrs.trim_end().ends_with('/') {
            if let Some(close_start) = markdown[end..].find("</pax-example>") {
                end += close_start + "</pax-example>".len();
            }
        }

        out.push(ParsedExampleEmbed { embed, start, end });
        cursor = end;
    }

    out
}

fn has_tag_boundary(markdown: &str, index: usize) -> bool {
    markdown[index..]
        .chars()
        .next()
        .map(|ch| ch.is_whitespace() || ch == '>' || ch == '/')
        .unwrap_or(true)
}

fn find_tag_end(markdown: &str, start: usize) -> Option<usize> {
    let mut quote = None;
    for (offset, ch) in markdown[start..].char_indices() {
        match (quote, ch) {
            (Some(active), current) if current == active => quote = None,
            (None, '"' | '\'') => quote = Some(ch),
            (None, '>') => return Some(start + offset),
            _ => {}
        }
    }
    None
}

fn parse_embed_attrs(attrs: &str) -> Option<ExampleEmbed> {
    let attrs = parse_attrs(attrs);
    let path = attrs
        .get("path")
        .and_then(|value| normalize_relative_path(value))?;
    let title = attrs
        .get("title")
        .cloned()
        .filter(|value| !value.is_empty());
    let height = attrs
        .get("height")
        .and_then(|value| value.parse::<u32>().ok());
    let files = attrs
        .get("files")
        .map(|value| {
            value
                .split(',')
                .filter_map(|file| normalize_relative_path(file.trim()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let inline_source = is_truthy(attrs.get("inline-source"))
        || is_truthy(attrs.get("terminal-source"))
        || is_truthy(attrs.get("source"));

    Some(ExampleEmbed {
        path,
        title,
        height,
        files,
        inline_source,
    })
}

fn parse_attrs(attrs: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let chars = attrs.char_indices().collect::<Vec<_>>();
    let mut cursor = 0usize;

    while cursor < chars.len() {
        while cursor < chars.len() && chars[cursor].1.is_whitespace() {
            cursor += 1;
        }
        if cursor >= chars.len() || chars[cursor].1 == '/' {
            break;
        }

        let key_start = chars[cursor].0;
        while cursor < chars.len() && is_attr_key_char(chars[cursor].1) {
            cursor += 1;
        }
        let key_end = chars
            .get(cursor)
            .map(|(index, _)| *index)
            .unwrap_or(attrs.len());
        if key_end == key_start {
            cursor += 1;
            continue;
        }
        let key = attrs[key_start..key_end].trim();
        if key.is_empty() {
            cursor += 1;
            continue;
        }

        while cursor < chars.len() && chars[cursor].1.is_whitespace() {
            cursor += 1;
        }

        if cursor >= chars.len() || chars[cursor].1 != '=' {
            out.insert(key.to_string(), "true".to_string());
            continue;
        }
        cursor += 1;

        while cursor < chars.len() && chars[cursor].1.is_whitespace() {
            cursor += 1;
        }

        if cursor >= chars.len() {
            out.insert(key.to_string(), String::new());
            break;
        }

        let value = if matches!(chars[cursor].1, '"' | '\'') {
            let quote = chars[cursor].1;
            cursor += 1;
            let value_start = chars
                .get(cursor)
                .map(|(index, _)| *index)
                .unwrap_or(attrs.len());
            while cursor < chars.len() && chars[cursor].1 != quote {
                cursor += 1;
            }
            let value_end = chars
                .get(cursor)
                .map(|(index, _)| *index)
                .unwrap_or(attrs.len());
            if cursor < chars.len() {
                cursor += 1;
            }
            attrs[value_start..value_end].to_string()
        } else {
            let value_start = chars[cursor].0;
            while cursor < chars.len() && !chars[cursor].1.is_whitespace() && chars[cursor].1 != '/'
            {
                cursor += 1;
            }
            let value_end = chars
                .get(cursor)
                .map(|(index, _)| *index)
                .unwrap_or(attrs.len());
            attrs[value_start..value_end].to_string()
        };

        out.insert(key.to_string(), value);
    }

    out
}

fn is_attr_key_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':')
}

fn is_truthy(value: Option<&String>) -> bool {
    value
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "true" | "1" | "yes" | "source" | "inline"
            )
        })
        .unwrap_or(false)
}

fn normalize_relative_path(path: &str) -> Option<String> {
    let raw = path.trim().replace('\\', "/");
    if raw.is_empty() || raw.starts_with('/') || raw.contains(':') {
        return None;
    }
    let normalized = raw.trim_matches('/');

    let mut parts = Vec::new();
    for part in normalized.split('/') {
        if part.is_empty() || matches!(part, "." | "..") {
            return None;
        }
        parts.push(part);
    }
    Some(parts.join("/"))
}

fn render_cli_fallback(embed: &ExampleEmbed, workspace: &Path) -> String {
    let title = embed
        .title
        .clone()
        .unwrap_or_else(|| humanize_example_title(&embed.path));
    let mut out = String::new();
    out.push_str(&format!("### Example: {title}\n\n"));
    out.push_str(&format!(
        "Hosted example: <{}>\n\n",
        hosted_example_url(&embed.path)
    ));
    out.push_str("From the Pax repository root (requires a source checkout):\n\n");
    out.push_str("```sh\n");
    out.push_str(&run_command(&embed.path));
    out.push_str("\n```\n");

    let Some(example_dir) = example_dir(workspace, &embed.path) else {
        return out;
    };

    let source_paths = source_file_relative_paths(&example_dir, &embed.files);
    if source_paths.is_empty() {
        return out;
    }

    out.push_str("\nSource files:\n");
    for source_path in &source_paths {
        out.push_str(&format!(
            "- `examples/src/{}/{}`\n",
            embed.path,
            source_path.to_string_lossy().replace('\\', "/")
        ));
    }

    if embed.inline_source {
        for source in read_source_files(&example_dir, &embed.files) {
            out.push_str(&format!("\n#### `{}`\n\n", source.path));
            out.push_str(&format!("```{}\n", source.language));
            out.push_str(&source.contents);
            if !source.contents.ends_with('\n') {
                out.push('\n');
            }
            if source.truncated {
                out.push_str("\n// ... truncated for docs output\n");
            }
            out.push_str("```\n");
        }
    }

    out
}

fn render_source_markdown(path: &str, title: String, source_files: &[ExampleSourceFile]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Example: {title}\n\n"));
    out.push_str(&format!("Path: `examples/src/{path}`\n\n"));
    out.push_str("From the Pax repository root (requires a source checkout):\n\n");
    out.push_str("```sh\n");
    out.push_str(&run_command(path));
    out.push_str("\n```\n");

    out.push_str("\nFiles:\n");
    for source in source_files {
        out.push_str(&format!("- `{}`\n", source.path));
    }

    for source in source_files {
        out.push_str(&format!("\n## `{}`\n\n", source.path));
        out.push_str(&format!("```{}\n", source.language));
        out.push_str(&source.contents);
        if !source.contents.ends_with('\n') {
            out.push('\n');
        }
        if source.truncated {
            out.push_str("\n// ... truncated for docs output\n");
        }
        out.push_str("```\n");
    }

    out
}

fn default_source_file_paths(example_dir: &Path) -> Vec<PathBuf> {
    let src_dir = example_dir.join("src");
    let mut discovered = Vec::new();
    collect_source_paths(&src_dir, example_dir, &mut discovered);
    discovered.sort();

    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for preferred in ["src/lib.pax", "src/lib.rs"] {
        let preferred = PathBuf::from(preferred);
        if discovered.contains(&preferred) && seen.insert(preferred.clone()) {
            out.push(preferred);
        }
    }

    for path in discovered {
        if out.len() >= MAX_DEFAULT_SOURCE_FILES {
            break;
        }
        if seen.insert(path.clone()) {
            out.push(path);
        }
    }

    out
}

fn collect_source_paths(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_source_paths(&path, base, out);
            continue;
        }
        if !file_type.is_file() || !is_source_file_path(&path) {
            continue;
        }
        if let Ok(relative) = path.strip_prefix(base) {
            out.push(relative.to_path_buf());
        }
    }
}

fn is_source_file_path(path: impl AsRef<Path>) -> bool {
    matches!(
        path.as_ref().extension().and_then(|ext| ext.to_str()),
        Some("pax" | "rs")
    )
}

fn read_source_file(example_dir: &Path, relative_path: &Path) -> Option<ExampleSourceFile> {
    let full_path = example_dir.join(relative_path);
    let mut contents = fs::read_to_string(full_path).ok()?;
    let truncated = truncate_string(&mut contents, MAX_SOURCE_BYTES);
    Some(ExampleSourceFile {
        path: relative_path.to_string_lossy().replace('\\', "/"),
        language: source_language(relative_path),
        contents,
        truncated,
    })
}

fn source_language(path: &Path) -> String {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("pax") => "pax".to_string(),
        Some("rs") => "rust".to_string(),
        Some(ext) => ext.to_string(),
        None => "text".to_string(),
    }
}

fn truncate_string(contents: &mut String, max_bytes: usize) -> bool {
    if contents.len() <= max_bytes {
        return false;
    }

    let mut end = max_bytes;
    while !contents.is_char_boundary(end) {
        end -= 1;
    }
    contents.truncate(end);
    true
}

pub fn fingerprint_dir(dir: &Path) -> io::Result<String> {
    let mut files = Vec::new();
    collect_fingerprint_files(dir, dir, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hash = 1469598103934665603u64;
    for (relative_path, contents) in files {
        hash = fnv1a(hash, relative_path.as_bytes());
        hash = fnv1a(hash, &contents);
    }

    Ok(format!("{hash:016x}"))
}

fn collect_fingerprint_files(
    base: &Path,
    dir: &Path,
    out: &mut Vec<(String, Vec<u8>)>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if matches!(name.as_ref(), ".git" | ".pax" | "target" | "node_modules") {
                continue;
            }
            collect_fingerprint_files(base, &path, out)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let relative_path = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let contents = fs::read(&path)?;
        out.push((relative_path, contents));
    }

    Ok(())
}

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}
