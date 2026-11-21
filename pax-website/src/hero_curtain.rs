use curtain_panel::{advance_draft, Sateen, DEFAULT_DRAFT_SPEED};
use pax_kit::*;

#[pax]
#[file("hero_curtain.pax")]
pub struct HeroCurtain {
    pub light_x: Property<f64>,
    pub light_y: Property<f64>,
    pub brightness: Property<f64>,
    pub phase: Property<f64>,
    pub visible: Property<bool>,
    pub last_ms: Property<f64>,
}

impl HeroCurtain {
    pub fn mount(&mut self, _: &NodeContext) {
        self.phase.set(Sateen::INITIAL_PHASE);
    }

    pub fn visibility(&mut self, _: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set_if_neq(event.current.is_in_viewport());
    }

    pub fn reset_clock(&mut self, ctx: &NodeContext) {
        self.last_ms.set(ctx.elapsed_time_millis() as f64);
    }

    pub fn animate(&mut self, ctx: &NodeContext) {
        advance_draft(ctx, &self.last_ms, &self.phase, DEFAULT_DRAFT_SPEED);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_curtain_clock_only_mounts_for_visible_gpu_paint() {
        let paint = include_str!("hero_curtain.pax");
        assert!(paint.contains("if @gpu && self.visible {"));
        assert_eq!(paint.matches("@tick=self.animate").count(), 1);
        assert_eq!(paint.matches("<Sateen ").count(), 1);
        assert!(!paint.contains("<Impressionist"));
        assert!(!paint.contains("<Text"));
        assert!(!paint.contains("slot("));
    }
}
