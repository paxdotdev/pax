use crate::SiteTheme;
use pax_kit::*;

/// An unframed study whose caller supplies a live app or linked capture.
/// The live viewport grows with the host width; scrolling never resizes it.
#[pax]
#[file("framed_study.pax")]
pub struct FramedStudy {
    pub url: Property<String>,
    pub label: Property<String>,
    pub detail: Property<String>,
    pub compact: Property<bool>,
    pub scene_progress: Property<f64>,
    pub content_height: Property<f64>,
    pub stage_width: Property<f64>,
    pub stage_height: Property<f64>,
    pub stage_scale: Property<f64>,
}

impl FramedStudy {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let compact = self.compact.local();
        let deps = [bounds.untyped(), compact.untyped()];
        let stage_width = self.stage_width.local();
        let scale = self.stage_scale.local();
        let height = self.stage_height.local();
        let content = self.content_height.local();
        let update = move || {
            let width = bounds.get().0;
            let width = if width.is_finite() {
                width.max(0.0)
            } else {
                0.0
            };
            let logical_width = study_width(width, compact.get());
            stage_width.set_if_neq(logical_width);
            scale.set_if_neq(width / logical_width * 100.0);
            height.set_if_neq(width * 600.0 / logical_width);
            content.set_if_neq(width * 600.0 / logical_width + 104.0);
        };
        update();
        ctx.subscribe(&deps, update);
    }
}

fn study_width(width: f64, compact: bool) -> f64 {
    // Keep the capture's original aspect ratio. Live desktops gain LED columns
    // before scaling up, with a bounded logical surface and readable controls.
    if compact {
        1000.0
    } else {
        width.clamp(1000.0, 1600.0)
    }
}

#[cfg(test)]
mod tests {
    use super::study_width;

    #[test]
    fn frame_projects_one_content_slot_and_keeps_motion_out_of_layout() {
        let template = include_str!("framed_study.pax");
        assert_eq!(template.matches("slot()").count(), 1);
        assert!(!template.contains("<NativeImage"));
        assert!(template.contains("height=600px"));
        assert!(!template.contains("<DeviceFrame"));
        assert!(template.contains("rotate=0deg"));
        assert!(!template.contains("rotate:"));
        assert!(template.contains("layout_role=LayoutRole::Breakout"));
        assert!(template.contains("playhead: {self.scene_progress}"));
    }

    #[test]
    fn desktop_scope_is_live_but_compact_scope_is_a_linked_capture() {
        let section = include_str!("framework_section.pax");
        assert_eq!(section.matches("<LedOscilloscope").count(), 1);
        assert!(section.contains("if self.compact"));
        assert!(section.contains("id=live_oscilloscope"));
        assert_eq!(section.matches("<NativeImage").count(), 1);
        assert!(!section.contains("<SpaceGame"));
        assert!(section.contains("url=\"/oscilloscope/\""));
        assert!(!section.contains("space-game"));
        assert_eq!(section.matches("<FramedStudy").count(), 1);
        assert!(section.contains("<FramedStudy width=100%"));
        assert!(section.contains("compact={self.compact}"));
    }

    #[test]
    fn wide_studies_gain_logical_width_before_scaling() {
        assert_eq!(study_width(800.0, false), 1000.0);
        assert_eq!(study_width(1200.0, false), 1200.0);
        assert_eq!(study_width(1600.0, false), 1600.0);
        assert_eq!(study_width(2400.0, false), 1600.0);
        for width in [320.0, 600.0, 880.0] {
            assert_eq!(study_width(width, true), 1000.0);
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
