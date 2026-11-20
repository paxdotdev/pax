use crate::SiteTheme;
use led_oscilloscope::LedOscilloscope;
use pax_kit::*;

/// An unframed live instrument with an independent full-screen link.
/// The live viewport grows with the host width; scrolling never resizes it.
#[pax]
#[file("framed_study.pax")]
pub struct FramedStudy {
    pub url: Property<String>,
    pub label: Property<String>,
    pub detail: Property<String>,
    pub scene_progress: Property<f64>,
    pub content_height: Property<f64>,
    pub stage_width: Property<f64>,
    pub stage_height: Property<f64>,
    pub logical_height: Property<f64>,
    pub stage_scale: Property<f64>,
}

impl FramedStudy {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
        let stage_width = self.stage_width.local();
        let scale = self.stage_scale.local();
        let height = self.stage_height.local();
        let logical_height = self.logical_height.local();
        let content = self.content_height.local();
        let update = move || {
            let width = bounds.get().0;
            let width = if width.is_finite() {
                width.max(0.0)
            } else {
                0.0
            };
            let (logical_width, logical_h) = study_size(width);
            let projected_height = width / logical_width * logical_h;
            stage_width.set_if_neq(logical_width);
            logical_height.set_if_neq(logical_h);
            scale.set_if_neq(width / logical_width * 100.0);
            height.set_if_neq(projected_height);
            content.set_if_neq(projected_height + 104.0);
        };
        update();
        ctx.subscribe(&deps, update);
    }
}

fn study_size(width: f64) -> (f64, f64) {
    // Keep touch controls at their natural size; only scale up on ultrawide screens.
    let width = width.clamp(1., 1600.);
    if width < 800. {
        (width, LedOscilloscope::height(width))
    } else {
        (width, (width * 0.6).min(600.))
    }
}

#[cfg(test)]
mod tests {
    use super::study_size;

    #[test]
    fn frame_projects_one_content_slot_and_keeps_motion_out_of_layout() {
        let template = include_str!("framed_study.pax");
        assert_eq!(template.matches("slot()").count(), 1);
        assert!(!template.contains("<NativeImage"));
        assert!(template.contains("height={(self.logical_height)px}"));
        assert!(!template.contains("<DeviceFrame"));
        assert!(template.contains("rotate=0deg"));
        assert!(!template.contains("rotate:"));
        assert!(template.contains("layout_role=LayoutRole::Breakout"));
        assert!(template.contains("playhead: {self.scene_progress}"));
    }

    #[test]
    fn both_form_factors_share_one_live_scope_and_a_separate_fullscreen_link() {
        let section = include_str!("framework_section.pax");
        assert_eq!(section.matches("<LedOscilloscope").count(), 1);
        assert!(!section.contains("if self.compact"));
        assert!(section.contains("id=live_oscilloscope"));
        assert!(!section.contains("<NativeImage"));
        assert!(!section.contains("<SpaceGame"));
        assert!(section.contains("url=\"/oscilloscope/\""));
        assert!(!section.contains("space-game"));
        assert_eq!(section.matches("<FramedStudy").count(), 1);
        assert!(section.contains("<FramedStudy width=100%"));
    }

    #[test]
    fn wide_studies_gain_logical_width_before_scaling() {
        assert_eq!(study_size(800.0), (800., 480.));
        assert_eq!(study_size(1200.0), (1200., 600.));
        assert_eq!(study_size(2400.0), (1600., 600.));
        for width in [320., 390., 440.] {
            let (w, h) = study_size(width);
            assert_eq!(w, width);
            assert!(h > w);
        }
    }

    #[test]
    fn space_game_is_not_shipped_or_routed_by_the_website() {
        assert!(!include_str!("../Cargo.toml").contains("space-game"));
        assert!(!include_str!("lib.pax").contains("/space-game"));
        assert!(!include_str!("playground_page.pax").contains("<SpaceGame"));
        assert!(!include_str!("route_pages.rs").contains("space_game::"));
    }
}
