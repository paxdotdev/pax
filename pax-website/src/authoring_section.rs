use crate::SiteTheme;
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
    fn the_hero_curtain_ends_above_the_black_language_and_cube_section() {
        let hero = include_str!("hero_section.pax");
        assert_eq!(hero.matches("<HeroCurtain").count(), 1);
        assert!(hero.contains("<HeroCurtain class=\"hero_sweep\" width=100% height=100%/>"));

        let section = include_str!("authoring_section.pax");
        assert!(!section.contains("<HeroCurtain"));
        assert!(!section.contains("<Impressionist"));
        assert!(!section.contains("<WetPane"));
        assert!(section.contains("fill={paint: BLACK, material: {Material::unlit()}}"));
        assert!(section.contains("id=authoring_intro"));
        assert!(section.contains("id=cube_stage"));
        assert!(section.contains("<ExampleHost"));
        assert!(section.contains("<ExampleSourceViewer"));
    }
}
