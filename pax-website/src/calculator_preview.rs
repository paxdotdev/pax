use crate::DeviceFrame;
use calculator::Calculator;
use pax_kit::*;

/// A live phone preview that mounts and updates with the page, including offscreen.
#[pax]
#[file("calculator_preview.pax")]
pub struct CalculatorPreview {
    pub content_height: Property<f64>,
    pub phone_scale: Property<f64>,
}

impl CalculatorPreview {
    pub fn layout(&mut self, ctx: &NodeContext) {
        let width = ctx.bounds_self.get().0;
        let scale = phone_scale(width);
        self.phone_scale.set_if_neq(scale * 100.0);
        self.content_height.set_if_neq(844.0 * scale);
    }
}

fn phone_scale(width: f64) -> f64 {
    // The surrounding footer owns its gutters; use the entire assigned column.
    (width / 390.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculator_mount_and_updates_have_no_visibility_gate() {
        let template = include_str!("calculator_preview.pax");
        assert!(!template.contains("if self.loaded"));
        assert!(!template.contains("@viewport_proximity"));
        assert!(template.contains("paused=false"));
    }

    #[test]
    fn phone_keeps_its_aspect_ratio_and_fits_its_assigned_column() {
        for width in [0.0, 160.0, 272.0, 342.0, 390.0, 768.0] {
            let scale = phone_scale(width);
            assert!(390.0 * scale <= width + 0.001);
            assert!(scale <= 1.0);
        }
    }
}
