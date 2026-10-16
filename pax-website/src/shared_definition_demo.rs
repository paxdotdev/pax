use crate::SiteTheme;
use pax_kit::*;

/// A small source/preview pair: both outputs read the same explicit input.
#[pax]
#[file("shared_definition_demo.pax")]
pub struct SharedDefinitionDemo {
    pub compact: Property<bool>,
    pub progress: Property<f64>,
    pub source: Property<String>,
}

impl SharedDefinitionDemo {
    pub fn mount(&mut self, _: &NodeContext) {
        self.progress.set(25.0);
        // Show the actual mounted template, not a second hand-maintained example.
        self.source
            .set(include_str!("relationship_preview.pax").trim().to_owned());
    }

    pub fn quarter(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.progress.set(25.0);
    }

    pub fn three_quarters(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.progress.set(75.0);
    }
}

#[pax]
#[file("relationship_preview.pax")]
pub struct RelationshipPreview {
    pub progress: Property<f64>,
}
