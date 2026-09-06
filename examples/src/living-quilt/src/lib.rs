use pax_kit::math::Point2;
use pax_kit::*;

pub mod animated_pax_logo;
pub mod animated_pax_logo_banner;
pub mod animated_pax_logo_post;
pub mod logo_card;
pub mod quilt_layer;
pub mod quilt_scene;
pub mod quilt_tile;
pub mod static_pax_logo;
pub use animated_pax_logo::*;
pub use animated_pax_logo_banner::*;
pub use animated_pax_logo_post::*;
pub use logo_card::*;
pub use quilt_layer::*;
pub use quilt_scene::*;
pub use quilt_tile::*;
pub use static_pax_logo::*;

const COMPACT_BREAKPOINT: f64 = 720.0;
const WINDING_DURATION_MS: u64 = 1280;
const REVEAL_DURATION_MS: u64 = 1280;
const LIGHT_TRAVEL_DURATION_MS: u64 = 420;

#[pax]
#[main]
#[custom(Default)]
#[file("lib.pax")]
pub struct Example {
    pub is_compact: Property<bool>,
    pub logo_progress: Property<f64>,
    pub light_x: Property<f64>,
    pub light_y: Property<f64>,
    pub spin_turn: Property<f64>,
    pub base_turn: Property<f64>,
    pub wave_origin: Property<usize>,
    pub base_generation: Property<usize>,
    pub next_generation: Property<usize>,
    pub ripples: Property<Vec<Ripple>>,
    pub next_ripple_id: Property<usize>,
}

#[pax]
#[custom(Defaults)]
pub struct Ripple {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub diameter: f64,
    pub progress: f64,
    pub started_at_ms: u64,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            is_compact: Property::new(false),
            logo_progress: Property::new(0.0),
            light_x: Property::new(0.5),
            light_y: Property::new(0.5),
            spin_turn: Property::new(0.0),
            base_turn: Property::new(0.0),
            wave_origin: Property::new(0),
            base_generation: Property::new(0),
            next_generation: Property::new(0),
            // Keeping one inert path in the mask subtree avoids a transient
            // zero-vertex stencil while keyed ripple children are replaced.
            ripples: Property::new(vec![ripple_sentinel()]),
            next_ripple_id: Property::new(1),
        }
    }
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);

        self.play_logo();
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
        self.advance_ripples(ctx);
    }

    pub fn handle_mouse_move(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        // MouseMove is desktop-only in the web chassis; touch taps continue
        // through the eased click path below.
        let (light_x, light_y) = normalized_point(ctx, event.mouse.x, event.mouse.y);
        self.light_x.set(light_x);
        self.light_y.set(light_y);
    }

    pub fn handle_tile_click(&mut self, ctx: &NodeContext, event: Event<Click>) {
        let (root_width, root_height) = ctx.bounds_self.get();
        if root_width <= 0.0 || root_height <= 0.0 {
            return;
        }

        let compact = self.is_compact.get();
        let columns = if compact { 3 } else { 5 };
        let rows = 15usize.div_ceil(columns);
        let (local_x, local_y) = normalized_point(ctx, event.mouse.x, event.mouse.y);
        let art_x = local_x * root_width;
        let art_y = local_y * root_height;
        let column = ((art_x / root_width) * columns as f64).floor() as usize;
        let row = ((art_y / root_height) * rows as f64).floor() as usize;
        let index = row.min(rows - 1) * columns + column.min(columns - 1);

        // Keep the touch-friendly move normalized so it survives a responsive
        // resize. For a mouse click this is already the tracked hover point.
        self.light_x.ease_to(
            local_x,
            Duration::Milliseconds(LIGHT_TRAVEL_DURATION_MS.into()),
            EasingCurve::OutQuad,
        );
        self.light_y.ease_to(
            local_y,
            Duration::Milliseconds(LIGHT_TRAVEL_DURATION_MS.into()),
            EasingCurve::OutQuad,
        );

        let settled_generation = self.next_generation.get();
        let starts_fresh_spin = spin_is_settled(self.spin_turn.get(), settled_generation);
        if starts_fresh_spin {
            self.base_generation.set(settled_generation);
            self.base_turn.set(settled_generation as f64);
            self.wave_origin.set(index);
            self.play_logo();
        }

        // Each impulse adds a separately expanding mask path. Earlier ripples
        // keep revealing the already-spinning quilt while later clicks wind it
        // up, instead of collapsing the reveal back to a point.
        let ripple_id = self.next_ripple_id.get();
        let mut ripples = self.ripples.get();
        if starts_fresh_spin {
            ripples.truncate(1);
        }
        ripples.push(Ripple {
            id: ripple_id,
            x: art_x,
            y: art_y,
            diameter: 2.0 * root_width.hypot(root_height) + 4.0,
            progress: 0.0,
            started_at_ms: elapsed_millis(ctx),
        });
        self.ripples.set(ripples);
        self.next_ripple_id.set(ripple_id + 1);

        // Retargeting checkpoints the current fractional turn. The full
        // accumulated remainder then completes in one fresh k-window, so rapid
        // impulses increase velocity while the final winding eases to rest.
        let target_generation = settled_generation + 1;
        self.next_generation.set(target_generation);
        self.spin_turn.ease_to(
            target_generation as f64,
            Duration::Milliseconds(WINDING_DURATION_MS.into()),
            EasingCurve::OutQuad,
        );
    }

    fn play_logo(&self) {
        self.logo_progress.cancel_transitions();
        self.logo_progress.set(0.0);
        self.logo_progress.ease_to(
            1.0,
            Duration::Milliseconds(1440.into()),
            EasingCurve::Linear,
        );
    }

    fn sync_layout(&mut self, ctx: &NodeContext) {
        let (width, _) = ctx.bounds_self.get();
        self.is_compact.set_if_neq(width < COMPACT_BREAKPOINT);
    }

    fn advance_ripples(&mut self, ctx: &NodeContext) {
        let now = elapsed_millis(ctx);
        let mut ripples = self.ripples.get();
        let mut changed = false;

        for ripple in &mut ripples {
            if ripple.id == 0 {
                continue;
            }
            let elapsed = now.saturating_sub(ripple.started_at_ms) as f64;
            let linear = (elapsed / REVEAL_DURATION_MS as f64).clamp(0.0, 1.0);
            let progress = ease_in_out_quad(linear);
            if (ripple.progress - progress).abs() > f64::EPSILON {
                ripple.progress = progress;
                changed = true;
            }
        }

        if changed {
            self.ripples.set(ripples);
        }
    }
}

fn ripple_sentinel() -> Ripple {
    Ripple {
        id: 0,
        x: 0.0,
        y: 0.0,
        diameter: 1.0,
        progress: 0.0,
        started_at_ms: 0,
    }
}

fn elapsed_millis(ctx: &NodeContext) -> u64 {
    ctx.elapsed_time_millis().min(u64::MAX as u128) as u64
}

fn normalized_point(ctx: &NodeContext, x: f64, y: f64) -> (f64, f64) {
    let local = ctx.local_point(Point2::new(x, y));
    (local.x.clamp(0.0, 1.0), local.y.clamp(0.0, 1.0))
}

fn ease_in_out_quad(progress: f64) -> f64 {
    if progress < 0.5 {
        2.0 * progress * progress
    } else {
        1.0 - (-2.0 * progress + 2.0).powi(2) / 2.0
    }
}

fn spin_is_settled(spin_turn: f64, target_generation: usize) -> bool {
    (spin_turn - target_generation as f64).abs() < 0.000_001
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spin_burst_settles_only_at_its_latest_target() {
        assert!(spin_is_settled(2.0, 2));
        assert!(!spin_is_settled(1.999, 2));
        assert!(!spin_is_settled(1.4, 2));
    }

    #[test]
    fn ripple_easing_preserves_endpoints_and_accelerates_then_brakes() {
        assert_eq!(ease_in_out_quad(0.0), 0.0);
        assert_eq!(ease_in_out_quad(0.5), 0.5);
        assert_eq!(ease_in_out_quad(1.0), 1.0);
        assert!(ease_in_out_quad(0.25) < 0.25);
        assert!(ease_in_out_quad(0.75) > 0.75);
    }
}
