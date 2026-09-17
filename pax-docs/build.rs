use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use tantivy::schema::{Schema, STORED, STRING, TEXT};
use tantivy::Index;

#[path = "src/examples.rs"]
#[allow(dead_code)]
mod examples;

#[path = "src/source_bundle.rs"]
mod source_bundle;

const MAGIC: &[u8; 8] = b"PAXDOCS\0";
const VERSION: u32 = 3;

#[derive(Clone, Copy)]
enum DocKind {
    Article,
    Api,
    Example,
}

impl DocKind {
    fn as_u8(self) -> u8 {
        match self {
            DocKind::Article => 0,
            DocKind::Api => 1,
            DocKind::Example => 2,
        }
    }
}

struct DocEntry {
    title: String,
    slug: String,
    kind: DocKind,
    depth: u8,
    path: String,
    body_markdown: String,
    summary: String,
    tags: Vec<String>,
}

struct IndexFile {
    path: String,
    bytes: Vec<u8>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let workspace = source_bundle::example_workspace(&manifest_dir, &out_dir)?;
    let workspace_dir = workspace.as_path();
    let book_dir = manifest_dir.join("book").join("src");
    let summary_path = book_dir.join("SUMMARY.md");

    println!("cargo:rerun-if-env-changed=PAX_DOCS_FORCE_REBUILD");
    println!("cargo:rerun-if-changed=bundled-example-sources.json");
    println!("cargo:rerun-if-changed={}", summary_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        workspace_dir.join("examples").join("src").display()
    );
    visit_markdown_files(&book_dir)?;
    visit_referenced_example_files(workspace_dir, &book_dir)?;
    visit_all_example_source_files(workspace_dir)?;

    let entries = load_entries(workspace_dir, &book_dir, &summary_path)?;
    let index_id = compute_docs_hash(&entries);

    let index_dir = out_dir.join("paxdocs_index");
    if index_dir.exists() {
        let _ = fs::remove_dir_all(&index_dir);
    }
    fs::create_dir_all(&index_dir)?;

    build_index(&index_dir, &entries)?;
    let mut index_files = Vec::new();
    collect_files(&index_dir, &index_dir, &mut index_files)?;
    index_files.sort_by(|a, b| a.path.cmp(&b.path));

    let out_file = out_dir.join("paxdocs.bin");
    write_paxdocs(&out_file, &index_id, &entries, &index_files)?;

    Ok(())
}

fn visit_markdown_files(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            visit_markdown_files(&path)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    Ok(())
}

fn visit_referenced_example_files(workspace_dir: &Path, book_dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(book_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some(examples::EXAMPLES_ASSET_DIR)
            {
                continue;
            }
            visit_referenced_example_files(workspace_dir, &path)?;
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let markdown = fs::read_to_string(&path)?;
        for embed in examples::discover_embeds(&markdown) {
            if let Some(example_dir) = examples::example_dir(workspace_dir, &embed.path) {
                let cargo_toml = example_dir.join("Cargo.toml");
                if cargo_toml.exists() {
                    println!("cargo:rerun-if-changed={}", cargo_toml.display());
                }
            }
            for source_path in examples::source_file_paths_for_embed(workspace_dir, &embed) {
                if source_path.exists() {
                    println!("cargo:rerun-if-changed={}", source_path.display());
                }
            }
        }
    }

    Ok(())
}

fn visit_all_example_source_files(workspace_dir: &Path) -> io::Result<()> {
    for example_path in examples::discover_example_paths(workspace_dir)? {
        let Some(example_dir) = examples::example_dir(workspace_dir, &example_path) else {
            continue;
        };
        let cargo_toml = example_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            println!("cargo:rerun-if-changed={}", cargo_toml.display());
        }

        for source_path in examples::source_file_relative_paths(&example_dir, &[]) {
            let source_path = example_dir.join(source_path);
            if source_path.exists() {
                println!("cargo:rerun-if-changed={}", source_path.display());
            }
        }
    }

    Ok(())
}

fn load_entries(
    workspace_dir: &Path,
    book_dir: &Path,
    summary_path: &Path,
) -> Result<Vec<DocEntry>, Box<dyn std::error::Error>> {
    let summary = fs::read_to_string(summary_path)?;
    let mut entries = Vec::new();

    for line in summary.lines() {
        if let Some((title, path, depth)) = parse_summary_line(line) {
            let full_path = book_dir.join(&path);
            let body_markdown = fs::read_to_string(&full_path)?;
            let body_markdown = examples::expand_examples_for_cli(&body_markdown, workspace_dir);
            let summary = extract_summary(&body_markdown).unwrap_or_default();
            let tags = extract_tags(&body_markdown);
            let slug = path_to_slug(&path);
            let kind = if slug.starts_with("api/") {
                DocKind::Api
            } else {
                DocKind::Article
            };

            entries.push(DocEntry {
                title,
                slug,
                kind,
                depth,
                path,
                body_markdown,
                summary,
                tags,
            });
        }
    }

    entries.extend(load_example_entries(workspace_dir)?);

    Ok(entries)
}

fn load_example_entries(workspace_dir: &Path) -> Result<Vec<DocEntry>, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();
    for example_path in examples::discover_example_paths(workspace_dir)? {
        let human_title = examples::humanize_example_title(&example_path);
        let title = format!("Example: {human_title}");
        let Some(body_markdown) = examples::render_example_source_markdown(
            workspace_dir,
            &example_path,
            Some(&human_title),
        ) else {
            continue;
        };
        let mut tags = vec![
            "examples".to_string(),
            "source".to_string(),
            example_path.clone(),
        ];
        tags.extend(
            example_path
                .split(['-', '_', '/'])
                .filter(|part| !part.is_empty())
                .map(|part| part.to_string()),
        );

        entries.push(DocEntry {
            title,
            slug: format!("examples/{example_path}"),
            kind: DocKind::Example,
            depth: 0,
            path: format!("examples/src/{example_path}"),
            body_markdown,
            summary: format!("Source files for `examples/src/{example_path}`."),
            tags,
        });
    }

    Ok(entries)
}

fn parse_summary_line(line: &str) -> Option<(String, String, u8)> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        return None;
    }

    if !(trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('+')) {
        return None;
    }

    let depth = summary_depth(line);
    let open = trimmed.find('[')?;
    let mid = trimmed[open + 1..].find("](")? + open + 1;
    let close = trimmed[mid + 2..].find(')')? + mid + 2;

    let title = trimmed[open + 1..mid].trim();
    let path = trimmed[mid + 2..close].trim();

    if title.is_empty() || path.is_empty() {
        return None;
    }

    Some((title.to_string(), path.to_string(), depth))
}

fn summary_depth(line: &str) -> u8 {
    let mut indent = 0usize;
    for ch in line.chars() {
        if ch == ' ' {
            indent += 1;
        } else if ch == '\t' {
            indent += 2;
        } else {
            break;
        }
    }
    (indent / 2) as u8
}

fn path_to_slug(path: &str) -> String {
    let mut slug = path.trim().trim_start_matches("./").to_string();
    if slug.ends_with(".md") {
        slug.truncate(slug.len() - 3);
    }
    slug
}

fn extract_summary(body: &str) -> Option<String> {
    if let Some(summary) = extract_metadata(body, "summary") {
        return Some(truncate_summary(&summary));
    }

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') || trimmed.starts_with("<!--") {
            continue;
        }
        return Some(truncate_summary(trimmed));
    }

    None
}

fn extract_tags(body: &str) -> Vec<String> {
    let Some(tags) = extract_metadata(body, "tags") else {
        return Vec::new();
    };

    tags.split(',')
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.to_string())
        .collect()
}

fn extract_metadata(body: &str, key: &str) -> Option<String> {
    let needle = format!("{key}:");
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(content) = trimmed
            .strip_prefix("<!--")
            .and_then(|value| value.strip_suffix("-->"))
        {
            let content = content.trim();
            if let Some(value) = content.strip_prefix(&needle) {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn truncate_summary(summary: &str) -> String {
    const LIMIT: usize = 120;
    if summary.len() <= LIMIT {
        return summary.to_string();
    }

    let mut out = summary[..LIMIT].trim_end().to_string();
    out.push_str("...");
    out
}

fn compute_docs_hash(entries: &[DocEntry]) -> String {
    let mut hash = 1469598103934665603u64;
    for entry in entries {
        hash = fnv1a(hash, entry.slug.as_bytes());
        hash = fnv1a(hash, &[entry.kind.as_u8()]);
        hash = fnv1a(hash, entry.title.as_bytes());
        hash = fnv1a(hash, entry.path.as_bytes());
        hash = fnv1a(hash, entry.summary.as_bytes());
        hash = fnv1a(hash, &[entry.depth]);
        for tag in &entry.tags {
            hash = fnv1a(hash, tag.as_bytes());
        }
        hash = fnv1a(hash, entry.body_markdown.as_bytes());
    }
    format!("{:016x}", hash)
}

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn build_index(index_dir: &Path, entries: &[DocEntry]) -> Result<(), Box<dyn std::error::Error>> {
    let mut schema_builder = Schema::builder();
    let title = schema_builder.add_text_field("title", TEXT | STORED);
    let body = schema_builder.add_text_field("body", TEXT);
    let tags = schema_builder.add_text_field("tags", TEXT | STORED);
    let slug = schema_builder.add_text_field("slug", STRING | STORED);
    let path = schema_builder.add_text_field("path", STRING | STORED);
    let entry_index = schema_builder.add_u64_field("entry_index", STORED);
    let schema = schema_builder.build();

    let index = Index::create_in_dir(index_dir, schema.clone())?;
    let mut writer = index.writer(50_000_000)?;

    for (idx, entry) in entries.iter().enumerate() {
        let mut doc = tantivy::TantivyDocument::default();
        doc.add_text(title, &entry.title);
        doc.add_text(body, &entry.body_markdown);
        doc.add_text(slug, &entry.slug);
        doc.add_text(path, &entry.path);
        for tag in &entry.tags {
            doc.add_text(tags, tag);
        }
        doc.add_u64(entry_index, idx as u64);
        writer.add_document(doc)?;
    }

    writer.commit()?;
    writer.wait_merging_threads()?;
    Ok(())
}

fn collect_files(base: &Path, dir: &Path, out: &mut Vec<IndexFile>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(base, &path, out)?;
        } else if path.is_file() {
            let rel = path.strip_prefix(base).unwrap();
            let mut rel_string = rel.to_string_lossy().to_string();
            if std::path::MAIN_SEPARATOR != '/' {
                rel_string = rel_string.replace(std::path::MAIN_SEPARATOR, "/");
            }
            let bytes = fs::read(&path)?;
            out.push(IndexFile {
                path: rel_string,
                bytes,
            });
        }
    }
    Ok(())
}

fn write_paxdocs(
    path: &Path,
    index_id: &str,
    entries: &[DocEntry],
    index_files: &[IndexFile],
) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(MAGIC)?;
    write_u32(&mut file, VERSION)?;
    write_string(&mut file, index_id)?;

    write_u32(&mut file, entries.len() as u32)?;
    for entry in entries {
        file.write_all(&[entry.kind.as_u8()])?;
        file.write_all(&[entry.depth])?;
        write_string(&mut file, &entry.slug)?;
        write_string(&mut file, &entry.title)?;
        write_string(&mut file, &entry.path)?;
        write_string(&mut file, &entry.summary)?;
        write_u32(&mut file, entry.tags.len() as u32)?;
        for tag in &entry.tags {
            write_string(&mut file, tag)?;
        }
        write_string(&mut file, &entry.body_markdown)?;
    }

    write_u32(&mut file, index_files.len() as u32)?;
    for index_file in index_files {
        write_string(&mut file, &index_file.path)?;
        write_bytes(&mut file, &index_file.bytes)?;
    }

    Ok(())
}

fn write_u32<W: Write>(writer: &mut W, value: u32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn write_string<W: Write>(writer: &mut W, value: &str) -> io::Result<()> {
    write_bytes(writer, value.as_bytes())
}

fn write_bytes<W: Write>(writer: &mut W, value: &[u8]) -> io::Result<()> {
    write_u32(writer, value.len() as u32)?;
    writer.write_all(value)
}
