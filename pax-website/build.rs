use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const AI_MARKDOWN_PATH: &str = "public/ai.md";
const AI_HTML_PATH: &str = "public/ai/index.html";

fn main() {
    println!("cargo:rerun-if-changed={AI_MARKDOWN_PATH}");
    println!("cargo:rerun-if-changed={AI_HTML_PATH}");
    println!("cargo:rerun-if-changed=build.rs");

    let project_root = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR")
            .expect("Cargo must provide CARGO_MANIFEST_DIR to the website build script"),
    );
    let markdown_path = project_root.join(AI_MARKDOWN_PATH);
    let html_path = project_root.join(AI_HTML_PATH);
    let markdown = fs::read_to_string(&markdown_path).unwrap_or_else(|err| {
        panic!(
            "Failed to read AI builder primer at {}: {err}",
            markdown_path.display()
        )
    });
    let html = render_ai_document(&markdown);

    write_if_changed(&html_path, html.as_bytes()).unwrap_or_else(|err| {
        panic!(
            "Failed to generate AI builder primer HTML at {}: {err}",
            html_path.display()
        )
    });
}

fn render_ai_document(markdown: &str) -> String {
    let escaped_markdown = escape_html(markdown);
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="description" content="Instructions for building Pax applications with an AI coding agent.">
  <title>Build with Pax</title>
  <link rel="alternate" type="text/markdown" href="/ai.md" title="Raw Markdown">
  <style>
    :root {{
      color-scheme: light dark;
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
        "Liberation Mono", "Courier New", monospace;
      background: #f6f5f2;
      color: #191919;
    }}

    * {{
      box-sizing: border-box;
    }}

    body {{
      margin: 0;
      min-height: 100vh;
      background:
        radial-gradient(circle at top left, rgba(103, 77, 255, 0.10), transparent 34rem),
        #f6f5f2;
    }}

    header,
    main {{
      width: min(100% - 2rem, 78rem);
      margin-inline: auto;
    }}

    header {{
      display: flex;
      justify-content: space-between;
      align-items: baseline;
      gap: 1rem;
      padding-block: 1.25rem;
      border-bottom: 1px solid rgba(25, 25, 25, 0.18);
    }}

    header strong {{
      font-size: 0.875rem;
      letter-spacing: 0.08em;
      text-transform: uppercase;
    }}

    a {{
      color: #5138d9;
      text-underline-offset: 0.2em;
    }}

    pre {{
      margin: 0;
      padding-block: clamp(2rem, 5vw, 4.5rem);
      white-space: pre-wrap;
      overflow-wrap: anywhere;
      font: inherit;
      font-size: clamp(0.875rem, 1.3vw, 1rem);
      line-height: 1.65;
      tab-size: 4;
    }}

    @media (prefers-color-scheme: dark) {{
      :root {{
        background: #121212;
        color: #ededed;
      }}

      body {{
        background:
          radial-gradient(circle at top left, rgba(126, 104, 255, 0.16), transparent 34rem),
          #121212;
      }}

      header {{
        border-bottom-color: rgba(237, 237, 237, 0.2);
      }}

      a {{
        color: #a99cff;
      }}
    }}
  </style>
</head>
<body>
  <header>
    <strong>Pax builder primer</strong>
    <a href="/ai.md">Raw Markdown</a>
  </header>
  <main>
    <pre>{escaped_markdown}</pre>
  </main>
</body>
</html>
"#
    )
}

fn escape_html(source: &str) -> String {
    let mut escaped = String::with_capacity(source.len());
    for character in source.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn write_if_changed(path: &Path, contents: &[u8]) -> io::Result<()> {
    if fs::read(path).ok().as_deref() == Some(contents) {
        return Ok(());
    }

    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Generated path has no parent: {}", path.display()),
        )
    })?;
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Generated path has no UTF-8 file name: {}", path.display()),
            )
        })?;
    let pending = parent.join(format!(".{file_name}.{}.pending", std::process::id()));
    fs::write(&pending, contents)?;

    let publish_result = match fs::rename(&pending, path) {
        Ok(()) => Ok(()),
        Err(_) if path.exists() => fs::remove_file(path).and_then(|_| fs::rename(&pending, path)),
        Err(err) => Err(err),
    };
    if publish_result.is_err() {
        let _ = fs::remove_file(pending);
    }
    publish_result
}

#[cfg(test)]
mod tests {
    use super::{escape_html, render_ai_document, write_if_changed};
    use std::fs;

    #[test]
    fn generated_document_contains_the_complete_escaped_markdown() {
        let markdown = "# Build <with> Pax\n\nUse `a && b`.\n";
        let html = render_ai_document(markdown);

        assert!(html.contains("<link rel=\"alternate\" type=\"text/markdown\" href=\"/ai.md\""));
        assert!(html.contains("# Build &lt;with&gt; Pax\n\nUse `a &amp;&amp; b`.\n"));
        assert!(!html.contains("# Build <with> Pax"));
    }

    #[test]
    fn escaping_preserves_quotes_for_plain_text_readability() {
        assert_eq!(escape_html("<&> \"Pax\""), "&lt;&amp;&gt; \"Pax\"");
    }

    #[test]
    fn generated_file_is_not_rewritten_when_contents_match() {
        let directory =
            std::env::temp_dir().join(format!("pax-website-build-test-{}", std::process::id()));
        let output = directory.join("ai/index.html");
        let _ = fs::remove_dir_all(&directory);

        write_if_changed(&output, b"first").unwrap();
        let first_modified = fs::metadata(&output).unwrap().modified().unwrap();
        write_if_changed(&output, b"first").unwrap();

        assert_eq!(fs::read(&output).unwrap(), b"first");
        assert_eq!(
            fs::metadata(&output).unwrap().modified().unwrap(),
            first_modified
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
