mod rules;

use crate::helpers::{replace_by_line_column, InlinedTemplateFinder};
use crate::{parse_pax_err, Rule};
use color_eyre::eyre::{self, Report, WrapErr};
use std::fs;
use std::path::{Path, PathBuf};
use syn::parse_file;
use syn::visit::Visit;

/// Result of formatting a file or directory tree.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FormatSummary {
    /// Pax-bearing source files inspected.
    pub files_checked: usize,
    /// Files whose canonical representation differs from disk.
    pub changed_files: Vec<PathBuf>,
}

/// Format one Pax template string.
pub fn format_pax_template(code: String) -> Result<String, eyre::Report> {
    let pax_component_definition = parse_pax_err(Rule::pax_component_definition, code.as_str())?;
    Ok(rules::format(pax_component_definition))
}

/// Format either a `.pax` file or an inlined Pax template inside a Rust file.
pub fn format_file(file_path: &str) -> Result<(), Report> {
    format_path(Path::new(file_path), false).map(|_| ())
}

/// Format a `.pax`/`.rs` file or every Pax-bearing source file below a directory.
///
/// Directory traversal skips generated and dependency directories (`.git`, `.pax`,
/// `target`, and `node_modules`). When `check` is true, files are inspected but not
/// written, and changed paths are returned in [`FormatSummary::changed_files`].
pub fn format_path(path: &Path, check: bool) -> Result<FormatSummary, Report> {
    if !path.exists() {
        return Err(Report::msg(format!(
            "format path does not exist: {}",
            path.display()
        )));
    }

    let mut files = Vec::new();
    collect_format_files(path, &mut files)?;
    files.sort();

    if files.is_empty() && path.is_file() {
        return Err(Report::msg("Unsupported file extension"));
    }

    let mut summary = FormatSummary::default();
    for file in files {
        let content = fs::read_to_string(&file)
            .wrap_err_with(|| format!("failed to read {}", file.display()))?;
        if file.extension().and_then(|extension| extension.to_str()) == Some("rs")
            && !content.contains("#[inlined")
        {
            continue;
        }

        summary.files_checked += 1;
        let formatted_content = format_source(&file, &content)
            .wrap_err_with(|| format!("failed to format {}", file.display()))?;
        if content != formatted_content {
            summary.changed_files.push(file.clone());
            if !check {
                fs::write(file, formatted_content)?;
            }
        }
    }

    Ok(summary)
}

fn collect_format_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), Report> {
    if path.is_file() {
        if matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("pax" | "rs")
        ) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            let directory_name = child.file_name().and_then(|name| name.to_str());
            if matches!(
                directory_name,
                Some(".git" | ".pax" | "target" | "node_modules")
            ) {
                continue;
            }
            collect_format_files(&child, files)?;
        } else if matches!(
            child.extension().and_then(|extension| extension.to_str()),
            Some("pax" | "rs")
        ) {
            files.push(child);
        }
    }

    Ok(())
}

fn format_source(path: &Path, content: &str) -> Result<String, Report> {
    let formatted = match path.extension().and_then(|extension| extension.to_str()) {
        Some("pax") => format_pax_template(content.to_string())?,
        Some("rs") => format_pax_in_rust_source(content)?,
        _ => return Err(Report::msg("Unsupported file extension")),
    };

    Ok(with_final_newline(formatted))
}

fn with_final_newline(mut content: String) -> String {
    while content.ends_with('\n') {
        content.pop();
        if content.ends_with('\r') {
            content.pop();
        }
    }
    content.push('\n');
    content
}

fn format_pax_in_rust_source(content: &str) -> Result<String, Report> {
    let ast = parse_file(&content)?;

    let mut finder = InlinedTemplateFinder::new(content.to_string());
    finder.visit_file(&ast);

    let mut modified_content = content.to_string();
    for template in finder.templates {
        let formatted_template = format_pax_template(template.template)?;
        let new_content = format!("(\n{}\n)", formatted_template);
        modified_content =
            replace_by_line_column(&modified_content, template.start, template.end, new_content)
                .unwrap();
    }
    Ok(modified_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_directory() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("pax-format-tests-{}-{unique}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn canonicalizes_repeated_static_classes_and_final_newline() {
        let directory = temp_directory();
        let path = directory.join("component.pax");
        fs::write(&path, "<Rectangle class=foo class=bar/>").unwrap();

        let check = format_path(&directory, true).unwrap();
        assert_eq!(check.changed_files, vec![path.clone()]);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "<Rectangle class=foo class=bar/>"
        );

        format_path(&directory, false).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "<Rectangle class=[foo, bar]/>\n"
        );
        assert!(format_path(&directory, true)
            .unwrap()
            .changed_files
            .is_empty());

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn recursively_formats_sources_but_skips_generated_directories() {
        let directory = temp_directory();
        let source = directory.join("src");
        let generated = directory.join("target");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&generated).unwrap();
        fs::write(source.join("component.pax"), "<Rectangle/>").unwrap();
        fs::write(source.join("plain.rs"), "fn main() {}").unwrap();
        fs::write(generated.join("generated.pax"), "<Rectangle/>").unwrap();

        let summary = format_path(&directory, false).unwrap();
        assert_eq!(summary.files_checked, 1);
        assert_eq!(
            fs::read_to_string(source.join("component.pax")).unwrap(),
            "<Rectangle />\n"
        );
        assert_eq!(
            fs::read_to_string(generated.join("generated.pax")).unwrap(),
            "<Rectangle/>"
        );
        assert_eq!(
            fs::read_to_string(source.join("plain.rs")).unwrap(),
            "fn main() {}"
        );

        fs::remove_dir_all(directory).unwrap();
    }
}
