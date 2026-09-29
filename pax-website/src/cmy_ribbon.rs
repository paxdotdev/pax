use crate::SiteTheme;
use pax_kit::*;

const RIBBON_WIDTH_PX: f64 = 6.0;

/// Parked, unlit CMY pinstripe treatment. Drive `progress` from zero to one
/// and place this content inside a Mask; it owns no animation clock or lights.
#[pax]
#[file("cmy_ribbon.pax")]
pub struct CmyRibbon {
    pub progress: Property<f64>,
    pub ribbon_x: Property<f64>,
    pub ribbon_scale: Property<f64>,
    pub ribbon_height: Property<f64>,
}

impl CmyRibbon {
    pub fn update(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        let pose = ribbon_pose(self.progress.get(), width, height);
        self.ribbon_x.set_if_neq(pose.x);
        self.ribbon_scale.set_if_neq(pose.scale);
        self.ribbon_height.set_if_neq(pose.height);
    }
}

struct RibbonPose {
    x: f64,
    scale: f64,
    height: f64,
}

fn ribbon_pose(progress: f64, width: f64, height: f64) -> RibbonPose {
    let progress = progress.clamp(0.0, 1.0);
    let em = height / 0.86;
    let ribbon_height = em * 3.5;
    let max_scale = (em * 0.3).max(1.0);
    let scale = 1.0 + (max_scale - 1.0) * (1.0 - (1.0 - progress).powi(3));
    // Begin fully outside, but finish against the mask's vertical slice so the
    // long off-mask ends do not make the visible sweep disappear prematurely.
    let start = -(ribbon_height + RIBBON_WIDTH_PX) * std::f64::consts::FRAC_1_SQRT_2 / 2.0 - 8.0;
    let end = width + half_span_in_mask(height, max_scale) + 4.0;
    RibbonPose {
        x: start + (end - start) * progress.powi(2),
        scale,
        height: ribbon_height,
    }
}

fn half_span_in_mask(height: f64, scale: f64) -> f64 {
    height / 2.0 + RIBBON_WIDTH_PX * scale / std::f64::consts::SQRT_2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ribbon_starts_fully_outside_and_clears_at_the_finish() {
        for em in [23.0, 30.0, 73.0, 90.0] {
            let height = em * 0.86;
            let width = height * 4060.8 / 815.55;
            let start = ribbon_pose(0.0, width, height);
            let initial_half_extent =
                (start.height + RIBBON_WIDTH_PX) * std::f64::consts::FRAC_1_SQRT_2 / 2.0;
            assert!(start.x + initial_half_extent < 0.0);
            let middle = ribbon_pose(0.5, width, height);
            assert!(middle.scale > start.scale * 5.0);
            let end = ribbon_pose(1.0, width, height);
            assert!(end.x - half_span_in_mask(height, end.scale) > width);
            let last_frame = ribbon_pose(1.0 - 16.0 / 500.0, width, height);
            assert!(last_frame.x - half_span_in_mask(height, last_frame.scale) < width);
            assert!(start.x < middle.x && middle.x < end.x);
        }
    }
}
