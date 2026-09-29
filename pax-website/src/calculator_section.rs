use crate::{DeviceFrame, SiteTheme};
use calculator::Calculator;
use pax_kit::*;

/// A live footer example: mount near the viewport once, then retain the session.
#[pax]
#[file("calculator_section.pax")]
pub struct CalculatorSection {
    pub content_height: Property<f64>,
    pub compact: Property<bool>,
    pub phone_scale: Property<f64>,
    pub phone_height: Property<f64>,
    pub loaded: Property<bool>,
    pub visible: Property<bool>,
}

impl CalculatorSection {
    pub fn layout(&mut self, ctx: &NodeContext) {
        let width = ctx.bounds_self.get().0;
        self.compact.set_if_neq(width < 900.0);
        let scale = phone_scale(width);
        self.phone_scale.set_if_neq(scale * 100.0);
        self.phone_height.set_if_neq(844.0 * scale);
        crate::section_layout::publish_height(ctx, "calculator_content", &self.content_height);
    }

    pub fn prepare(&mut self, _ctx: &NodeContext, event: Event<ViewportProximityEnter>) {
        self.loaded.set(true);
        self.visible.set(event.current.is_in_viewport());
    }

    pub fn visibility(&mut self, _ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set_if_neq(event.current.is_in_viewport());
    }
}

fn phone_scale(width: f64) -> f64 {
    ((width - 48.0) / 390.0).clamp(0.5, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_keeps_its_aspect_ratio_and_fits_mobile_gutters() {
        for width in [320.0, 390.0, 768.0, 1440.0] {
            let scale = phone_scale(width);
            assert!(390.0 * scale <= width - 48.0 + 0.001);
            assert!(scale <= 1.0);
        }
    }
}
