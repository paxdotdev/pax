use crate::{DeviceFrame, SiteTheme};
use pax_kit::*;

/// A fixed-footprint study whose caller supplies a live app or linked capture.
/// Scroll choreography transforms the frame without resizing the inner content.
#[pax]
#[file("framed_study.pax")]
pub struct FramedStudy {
    pub url: Property<String>,
    pub label: Property<String>,
    pub detail: Property<String>,
    pub kind: Property<String>,
    pub scene_progress: Property<f64>,
    pub mirrored: Property<bool>,
    pub content_height: Property<f64>,
    pub stage_height: Property<f64>,
    pub stage_scale: Property<f64>,
}

impl FramedStudy {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
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
            scale.set_if_neq(width / 1072.0 * 100.0);
            height.set_if_neq(width * 716.0 / 1072.0);
            content.set_if_neq(width * 716.0 / 1072.0 + 88.0);
        };
        update();
        ctx.subscribe(&deps, update);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn frame_projects_one_content_slot_and_keeps_motion_out_of_layout() {
        let template = include_str!("framed_study.pax");
        assert_eq!(template.matches("slot()").count(), 1);
        assert!(!template.contains("<NativeImage"));
        assert!(template.contains("width=1000px height=644px"));
        assert!(template.contains("layout_role=LayoutRole::Breakout"));
        assert!(template.contains("playhead: {self.scene_progress}"));
    }

    #[test]
    fn desktop_scope_is_live_but_compact_scope_and_space_game_are_linked_captures() {
        let section = include_str!("framework_section.pax");
        assert_eq!(section.matches("<LedOscilloscope").count(), 1);
        assert!(section.contains("if self.compact"));
        assert!(section.contains("id=live_oscilloscope"));
        assert_eq!(section.matches("<NativeImage").count(), 2);
        assert!(!section.contains("<SpaceGame"));
        assert!(section.contains("url=\"/oscilloscope/\""));
        assert!(section.contains("url=\"/space-game/\""));
    }
}
