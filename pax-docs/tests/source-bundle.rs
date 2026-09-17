#[path = "../src/examples.rs"]
#[allow(dead_code)]
mod examples;
#[path = "../src/source_bundle.rs"]
mod source_bundle;

use std::fs;
use std::path::Path;

#[test]
fn embedded_cli_index_contains_readable_example_sources() {
    let entries = pax_docs::entries().unwrap();
    let quilt = entries
        .iter()
        .find(|entry| entry.slug == "examples/living-quilt")
        .expect("registry-built CLI docs must include canonical example sources");
    assert_eq!(quilt.kind, pax_docs::DocKind::Example);
    assert!(quilt.body_markdown.contains("```rust"));
    assert!(quilt.body_markdown.contains("```pax"));
    assert!(!pax_docs::search("Living Quilt", 5).unwrap().is_empty());
}

#[test]
fn packaged_snapshot_preserves_example_catalog_and_inline_sources() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir().unwrap();
    let packaged = temp.path().join("pax-docs");
    fs::create_dir(&packaged).unwrap();
    fs::copy(
        manifest.join("bundled-example-sources.json"),
        packaged.join("bundled-example-sources.json"),
    )
    .unwrap();
    let unpacked = source_bundle::example_workspace(&packaged, &temp.path().join("out")).unwrap();
    let paths = examples::discover_example_paths(&unpacked).unwrap();
    assert!(paths.iter().any(|path| path == "living-quilt"));
    assert!(paths.iter().any(|path| path == "space-game"));
    let entries = pax_docs::entries().unwrap();
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.kind == pax_docs::DocKind::Example)
            .count(),
        paths.len()
    );
    for name in &paths {
        let expected = examples::render_example_source_markdown(
            &unpacked,
            name,
            Some(&examples::humanize_example_title(name)),
        )
        .unwrap();
        let entry = entries
            .iter()
            .find(|entry| entry.slug == format!("examples/{name}"))
            .expect("every source bundle example must reach the compiled index");
        assert_eq!(entry.body_markdown, expected);
    }

    let canonical = manifest.parent().unwrap();
    if canonical.join("examples/src").is_dir() {
        assert_eq!(paths, examples::discover_example_paths(canonical).unwrap());
        for name in &paths {
            assert_eq!(
                examples::render_example_source_markdown(&unpacked, name, None),
                examples::render_example_source_markdown(canonical, name, None),
                "stale source for {name}; run scripts/sync-docs-examples.py"
            );
        }
        for file in fs::read_dir(manifest.join("book/src")).unwrap() {
            let path = file.unwrap().path();
            if path.extension().is_some_and(|ext| ext == "md") {
                let body = fs::read_to_string(path).unwrap();
                assert_eq!(
                    examples::expand_examples_for_cli(&body, &unpacked),
                    examples::expand_examples_for_cli(&body, canonical)
                );
            }
        }
    }
}

#[test]
fn invalid_bundle_cannot_write_outside_its_source_directory() {
    let temp = tempfile::tempdir().unwrap();
    for name in [
        "../escape.rs",
        "examples/src/../escape.rs",
        "/absolute.rs",
        "examples/src/a\\escape.rs",
    ] {
        let bundle = serde_json::json!({"schema":1,"files":{name:"unsafe"}});
        assert!(
            source_bundle::materialize(&serde_json::to_vec(&bundle).unwrap(), temp.path()).is_err()
        );
    }
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    assert!(source_bundle::materialize(br#"{"schema":2,"files":{}}"#, temp.path()).is_err());
}

#[test]
fn new_snapshot_cannot_retain_removed_sources() {
    let temp = tempfile::tempdir().unwrap();
    let old = br#"{"schema":1,"files":{"examples/src/demo/old.rs":"old"}}"#;
    let new = br#"{"schema":1,"files":{"examples/src/demo/new.rs":"new"}}"#;
    let old_root = source_bundle::materialize(old, temp.path()).unwrap();
    let new_root = source_bundle::materialize(new, temp.path()).unwrap();
    assert_ne!(old_root, new_root);
    assert!(!new_root.join("examples/src/demo/old.rs").exists());
    assert_eq!(
        fs::read_to_string(new_root.join("examples/src/demo/new.rs")).unwrap(),
        "new"
    );
}
