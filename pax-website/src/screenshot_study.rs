use crate::{DeviceFrame, SiteTheme};
use pax_kit::*;

/// A fixed-footprint screenshot study. Only its frame pose responds to scroll;
/// the example itself belongs to the destination route, never this component.
#[pax]
#[file("screenshot_study.pax")]
pub struct ScreenshotStudy {
    pub image: Property<String>,
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

impl ScreenshotStudy {
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
    fn screenshot_study_has_no_live_example_and_keeps_motion_out_of_layout() {
        let template = include_str!("screenshot_study.pax");
        assert!(template.contains("<NativeImage"));
        assert!(template.contains("layout_role=LayoutRole::Breakout"));
        assert!(template.contains("playhead: {self.scene_progress}"));
        let section = include_str!("framework_section.pax");
        assert!(!section.contains("<LedOscilloscope"));
        assert!(!section.contains("<SpaceGame"));
        assert!(section.contains("url=\"/oscilloscope/\""));
        assert!(section.contains("url=\"/space-game/\""));
    }
}
