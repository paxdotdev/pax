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
    let mut article = String::new();
    let parser = pulldown_cmark::Parser::new(markdown).map(|event| match event {
        pulldown_cmark::Event::Html(text) | pulldown_cmark::Event::InlineHtml(text) => {
            pulldown_cmark::Event::Text(text)
        }
        other => other,
    });
    pulldown_cmark::html::push_html(&mut article, parser);
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="description" content="Instructions for building Pax applications with an AI coding agent.">
  <title>Build with Pax — AI guide</title>
  <link rel="canonical" href="https://www.pax.dev/ai/">
  <link rel="alternate" type="text/markdown" href="/ai.md" title="Raw Markdown">
  <style>
    :root {{
      color-scheme: dark;
      font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: #11110f;
      color: #faf8f0;
    }}

    * {{
      box-sizing: border-box;
    }}

    body {{
      margin: 0;
      min-height: 100vh;
    }}

    header,
    main {{
      width: min(100% - 3rem, 52rem);
      margin-inline: auto;
    }}

    header {{
      display: flex;
      justify-content: space-between;
      align-items: baseline;
      gap: 1rem;
      padding-block: 1.25rem;
      border-bottom: 1px solid #363633;
    }}

    header a {{
      font-size: 0.875rem;
      letter-spacing: 0.08em;
      text-transform: uppercase;
    }}

    a {{
      color: #faf8f0;
      text-underline-offset: 0.2em;
    }}

    a:focus-visible {{ outline: 2px solid #d2992b; outline-offset: 5px; }}
    main {{ padding-block: 2.5rem 5rem; font-size: 1.0625rem; line-height: 1.7; }}
    h1, h2 {{ line-height: 1.15; letter-spacing: -0.035em; }}
    h1 {{ font-size: clamp(2.25rem, 6vw, 3.5rem); margin-top: 0; }}
    h2 {{ font-size: 1.65rem; margin-top: 3rem; }}
    p, li {{ color: #bcbcb6; }}
    li + li {{ margin-top: 0.5rem; }}
    code {{ font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 0.9em; color: #faf8f0; }}
    pre {{
      padding: 1.25rem;
      background: #1a1a18;
      border: 1px solid #363633;
      border-radius: 12px;
      white-space: pre-wrap;
      overflow-wrap: anywhere;
      line-height: 1.6;
      tab-size: 4;
    }}

    @media print {{
      :root {{ color-scheme: light; background: white; color: black; }}
      a, p, li, code {{ color: black; }}
      pre {{ background: none; }}
    }}
  </style>
</head>
<body>
  <header>
    <a href="/" aria-label="Pax home">Pax / Build with AI</a>
    <a href="/ai.md">Raw Markdown</a>
  </header>
  <main>
    <article>{article}</article>
  </main>
</body>
</html>
"#
    )
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
    use super::{render_ai_document, write_if_changed};
    use std::fs;

    #[test]
    fn generated_document_has_semantic_headings_links_and_code() {
        let markdown = "# Build with Pax\n\nUse `a && b`.\n\n[Docs](https://docs.pax.dev/)\n";
        let html = render_ai_document(markdown);

        assert!(html.contains("<link rel=\"alternate\" type=\"text/markdown\" href=\"/ai.md\""));
        assert!(html.contains("<h1>Build with Pax</h1>"));
        assert!(html.contains("<code>a &amp;&amp; b</code>"));
        assert!(html.contains("<a href=\"https://docs.pax.dev/\">Docs</a>"));
    }

    #[test]
    fn raw_html_is_displayed_as_text_not_executed() {
        let html = render_ai_document("<script>alert('no')</script>");
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
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
