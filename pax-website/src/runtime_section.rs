#![allow(unused_imports)]

use crate::{runtime_glass::RuntimeGlass, PaxflixShowcase, SiteTheme};
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;
const BOTTOM_INSET: f64 = 24.0;

#[pax]
#[has_helpers]
#[file("runtime_section.pax")]
pub struct RuntimeSection {
    pub light_mode: Property<bool>,
    pub content_height: Property<f64>,
    pub _height_bound: Property<bool>,
    pub compact: Property<bool>,
    pub showcase_scale: Property<f64>,
    pub scene_progress: Property<f64>,
}

// The section allocates the stage and its Stacker row from the same width.
fn showcase_scale(section_width: f64, compact: bool) -> f64 {
    if !section_width.is_finite() {
        return 0.0;
    }
    let width = if compact {
        section_width - 48.0
    } else {
        section_width * 0.88
    };
    width.max(0.0) / 1280.0
}

pub(crate) const HEIGHT_SOURCE: &str = "runtime_content";

#[helpers]
impl RuntimeSection {
    pub fn stage_height(scale: f64, compact: bool) -> f64 {
        scale * 729.0 + if compact { 52.0 } else { 0.0 }
    }

    pub fn cell_sizes(scale: f64, compact: bool) -> Vec<Option<Size>> {
        vec![None, Some(Size::Pixels(Self::stage_height(scale, compact).into()))]
    }

    pub fn copy_inset(compact: bool) -> f64 {
        if compact { 56.0 } else { 80.0 }
    }
}

// The demo and editorial insets have an intrinsic height before any native
// text is measured. The live content measurement takes over at the actual
// preview width, including when a drawer changes the available space.
pub(crate) fn pending_section_height(width: f64) -> f64 {
    let compact = width < COMPACT_BREAKPOINT_PX;
    let scale = showcase_scale(width, compact);
    (RuntimeSection::stage_height(scale, compact)
        + 2.0 * RuntimeSection::copy_inset(compact)
        + BOTTOM_INSET)
        .ceil()
        .max(400.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_and_row_share_the_allocated_width_across_the_breakpoint() {
        for width in [0.0, 47.0, 48.0, f64::NAN, f64::INFINITY] {
            assert_eq!(showcase_scale(width, true), 0.0);
        }
        for width in [272.0, 759.0, 760.0, 1126.0, 2560.0] {
            let compact = width < COMPACT_BREAKPOINT_PX;
            let expected_width = if compact { width - 48.0 } else { width * 0.88 };
            assert!((showcase_scale(width, compact) * 1280.0 - expected_width).abs() < 0.001);
        }
    }

    #[test]
    fn glass_spans_the_runtime_and_modularity_uses_the_plain_site_background() {
        let runtime = include_str!("runtime_section.pax");
        let (content, background) = runtime.rsplit_once("</Stacker>").unwrap();
        let (intro, _) = content.split_once("<PaxflixShowcase").unwrap();
        assert!(intro.contains(">An alternative to HTML</Text>"));
        assert_eq!(runtime.matches("<RuntimeGlass ").count(), 1);
        assert!(background.contains("<RuntimeGlass "));
        assert!(!runtime.contains("<ModularityRain"));

        let modularity = include_str!("modularity_section.pax");
        let (content, background) = modularity.rsplit_once("</Stacker>").unwrap();
        assert!(content.contains(">Plug, Extend, and Play</Text>"));
        assert!(content.contains("<CircuitBoard"));
        assert!(!modularity.contains("<ModularityRain"));
        assert!(background.contains("<Rectangle class=\"site_ink\""));

        for (paint, shader) in [
            (include_str!("runtime_glass.pax"), "GlassBricks"),
            (include_str!("modularity_rain.pax"), "WetPane"),
        ] {
            let opening = format!("<{shader} ");
            let closing = format!("</{shader}>");
            let captured = paint
                .split_once(&opening)
                .unwrap()
                .1
                .split_once(&closing)
                .unwrap()
                .0;
            assert!(captured.contains("<Rectangle"));
            assert!(!captured.contains("<Text"));
            assert!(!captured.contains("slot("));
        }
    }

    #[test]
    fn paxflix_has_its_own_row_and_calculator_is_not_on_the_homepage() {
        let runtime = include_str!("runtime_section.pax");
        assert_eq!(runtime.matches("<PaxflixShowcase").count(), 1);
        assert!(!runtime.contains("<CalculatorPreview"));
        let authoring = include_str!("authoring_section.pax");
        assert!(!authoring.contains("<CalculatorPreview"));
        let demos = include_str!("framework_section.pax");
        assert!(!demos.contains("<CalculatorPreview"));
        assert_eq!(demos.matches("<LedOscilloscope").count(), 1);
        assert!(!include_str!("resource_section.pax").contains("<CalculatorPreview"));
        // Removing the homepage showcase must not break existing demo URLs.
        assert!(include_str!("calculator_page.pax").contains("<Calculator"));
    }

    #[test]
    fn hero_introduces_rust_in_the_supporting_copy_not_a_third_heading_line() {
        let headline = include_str!("rotating_headline.pax");
        assert!(!headline.contains(">with Rust</Text>"));
        let hero = include_str!("hero_section.pax");
        assert!(hero.contains(">Declare UIs in Pax and write logic in Rust.</Text>"));
        assert!(hero
            .contains(">Ship to native mobile, desktop, and web from a single codebase.</Text>"));
        assert!(hero.contains("self.font_px * 2.52"));
    }
}

impl RuntimeSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self._height_bound.set(false);
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
        let compact_bounds = bounds.clone();
        self.compact.replace_with(LocalProperty::computed_with_cutoff(
            move || compact_bounds.get().0 < COMPACT_BREAKPOINT_PX,
            &deps,
            PartialEq::eq,
        ));
        let compact = self.compact.local();
        let deps = [bounds.untyped(), compact.untyped()];
        self.showcase_scale.replace_with(LocalProperty::computed(
            move || showcase_scale(bounds.get().0, compact.get()),
            &deps,
        ));
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        if !self._height_bound.get() {
            self._height_bound
                .set(crate::section_layout::bind_height_with_padding(
                    ctx,
                    HEIGHT_SOURCE,
                    &self.content_height,
                    &Property::new(BOTTOM_INSET),
                ));
        }
    }
}
