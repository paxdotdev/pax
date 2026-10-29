//! Run `cargo run -p pax-language --release --example bench_text_bodies`.
//! PAX_TEXT_BENCH_MAX limits the largest payload in bytes (default 4 MiB).
//! Reports median wall time and allocation traffic, not retained memory.
use pax_language::{formatting::format_pax_template, parse_pax_str, Rule};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

struct Counting;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(size, Ordering::Relaxed);
        System.realloc(ptr, layout, size)
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn measure(name: &str, source: &str, valid: bool, format: bool) {
    let mut samples = Vec::with_capacity(5);
    for _ in 0..6 {
        ALLOCATIONS.store(0, Ordering::Relaxed);
        BYTES.store(0, Ordering::Relaxed);
        let start = Instant::now();
        if format {
            let result = format_pax_template(source.to_owned());
            assert_eq!(result.is_ok(), valid, "{name}");
            std::hint::black_box(result).ok();
        } else {
            let result = parse_pax_str(Rule::pax_component_definition, source);
            assert_eq!(result.is_ok(), valid, "{name}");
            std::hint::black_box(result).ok();
        }
        let sample = (
            start.elapsed().as_micros(),
            ALLOCATIONS.load(Ordering::Relaxed),
            BYTES.load(Ordering::Relaxed),
        );
        if samples.len() == 5 {
            samples.remove(0);
        }
        samples.push(sample);
    }
    samples.sort();
    let (micros, allocations, bytes) = samples[2];
    println!(
        "{name},{},{},{micros},{allocations},{bytes}",
        source.len(),
        if format { "format" } else { "parse" }
    );
}

fn main() {
    let max: usize = std::env::var("PAX_TEXT_BENCH_MAX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4 * 1024 * 1024);
    println!("case,source_bytes,operation,median_us,allocations,allocated_bytes");
    for power in 10..=22 {
        let size = 1 << power;
        if size > max {
            break;
        }
        for (name, unit, raw) in [
            ("prose", "A paragraph of ordinary prose. ", false),
            (
                "markdown",
                "## Heading\n\n**bold** and *emphasis*  \n\n",
                false,
            ),
            ("references", "&lt;/Text&gt; &#123; &amp; &#x1f680; ", false),
            ("unicode", "é 🚀 é 中文 \n", false),
            ("raw_delimiters", "</Text><Text> {x} & \"#### \\n\n", true),
        ] {
            let body = unit.repeat(size / unit.len() + 1);
            let content = if raw {
                format!("{{r#####\"{body}\"#####}}")
            } else {
                body
            };
            let source = format!("<Text>{content}</Text>");
            measure(name, &source, true, false);
            measure(name, &source, true, true);
            measure(
                &format!("{name}_missing_close"),
                &format!("<Text>{content}"),
                false,
                false,
            );
        }
    }
    for depth in [8, 32, 128] {
        let source = format!(
            "{}<Text>hello</Text>{}",
            "<Group>".repeat(depth),
            "</Group>".repeat(depth)
        );
        measure(&format!("nesting_{depth}"), &source, true, false);
        measure(
            &format!("nesting_{depth}_missing_close"),
            &source[..source.len() - 8],
            false,
            false,
        );
    }
    for size in [1024, 16384, 262144] {
        let source = format!("<Text>{{r###\"{}</Text>", "\"## x ".repeat(size / 6));
        measure("missing_raw_terminator", &source, false, false);
        let tag = format!("T{}", "a".repeat(size));
        let source = format!("<{tag}>hello</{tag}>");
        measure("long_tag", &source, true, false);
        let source = "<Text>hello</Text>".repeat(size / 17);
        measure("many_bodies", &source, true, false);
        measure("many_bodies", &source, true, true);
    }
}
