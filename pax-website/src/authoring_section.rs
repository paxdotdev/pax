use crate::{LanguageBackdrop, SiteTheme};
use increment_cube::IncrementCube;
use pax_kit::*;

#[pax]
#[file("authoring_section.pax")]
pub struct AuthoringSection {
    pub content_height: Property<f64>,
    pub _height_bound: Property<bool>,
    pub compact: Property<bool>,
    pub sources: Property<Vec<ExampleSource>>,
    pub selected_source: Property<usize>,
    pub source_light: Property<bool>,
    pub source_open: Property<bool>,
    pub demo_open: Property<bool>,
    pub cube_count: Property<u64>,
}

impl AuthoringSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self._height_bound.set(false);
        self.demo_open.set(true);
        crate::section_layout::bind_compact(ctx, &self.compact, 900.0);
    }

    pub fn expand_source(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.source_open.set(true);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        if !self._height_bound.get() {
            self._height_bound
                .set(crate::section_layout::bind_height_with_padding(
                    ctx,
                    "authoring_content",
                    &self.content_height,
                    &Property::new(0.0),
                ));
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn painterly_backdrop_stops_before_the_cube_stage() {
        let template = include_str!("authoring_section.pax");
        assert_eq!(template.matches("<LanguageBackdrop").count(), 1);
        assert!(!template.contains("<WetPane"));
        assert!(!template.contains("Fluent in UI"));
        let intro = template.find("id=authoring_intro").unwrap();
        let intro_end = intro + template[intro..].find("</Group>").unwrap();
        let backdrop = template.find("<LanguageBackdrop").unwrap();
        let cube = template.find("id=cube_stage").unwrap();
        assert!(intro < backdrop && backdrop < intro_end && intro_end < cube);
        assert!(template[backdrop..intro_end].contains("layout_role=LayoutRole::Breakout"));
        assert!(template.contains("<Rectangle class=\"site_ink\" width=100% height=100%/>"));
        assert!(template.contains("<ExampleHost"));
        assert!(template.contains("<ExampleSourceViewer"));
    }
}
