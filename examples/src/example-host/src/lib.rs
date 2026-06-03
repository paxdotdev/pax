#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub sources: Property<Vec<ExampleSource>>,
}

#[pax]
#[file("tile.pax")]
pub struct ExampleTile {
    pub label: Property<String>,
    pub detail: Property<String>,
    pub color: Property<Color>,
}

impl Example {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        self.sources.set(vec![
            ExampleSource {
                label: "src/lib.pax".to_string(),
                language: "pax".to_string(),
                code: include_str!("lib.pax").to_string(),
            },
            ExampleSource {
                label: "src/tile.pax".to_string(),
                language: "pax".to_string(),
                code: include_str!("tile.pax").to_string(),
            },
            ExampleSource {
                label: "src/lib.rs".to_string(),
                language: "rust".to_string(),
                code: include_str!("lib.rs").to_string(),
            },
        ]);
    }
}
