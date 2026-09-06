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
pub mod wave;
pub use animated_pax_logo::*;
pub use animated_pax_logo_banner::*;
pub use animated_pax_logo_post::*;
pub use logo_card::*;
pub use quilt_layer::*;
pub use quilt_scene::*;
pub use quilt_tile::*;
pub use static_pax_logo::*;
pub use wave::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub is_compact: Property<bool>,
    pub logo_progress: Property<f64>,
    pub light_x: Property<f64>,
    pub light_y: Property<f64>,
    pub tiles: Property<Vec<TileState>>,
    pub ripples: Property<Vec<Ripple>>,
    pub next_ripple_id: Property<usize>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.is_compact.set(ctx.bounds_self.get().0 < 720.0);
        self.light_x.set(0.5);
        self.light_y.set(0.5);
        self.tiles.set(
            (0..15)
                .map(|id| TileState {
                    id,
                    panels: vec![Panel::new(0, rand::random(), 0, true)],
                })
                .collect(),
        );
        self.next_ripple_id.set(1);
        self.play_logo();
    }

    pub fn handle_mouse_move(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        let (x, y) = normalized_point(ctx, event.mouse.x, event.mouse.y);
        self.light_x.set(x);
        self.light_y.set(y);
    }

    pub fn handle_tile_click(&mut self, ctx: &NodeContext, event: Event<Click>) {
        let (width, height) = ctx.bounds_self.get();
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let (x, y) = normalized_point(ctx, event.mouse.x, event.mouse.y);
        self.light_x
            .ease_to(x, Duration::Milliseconds(420.into()), EasingCurve::OutQuad);
        self.light_y
            .ease_to(y, Duration::Milliseconds(420.into()), EasingCurve::OutQuad);
        let mut ripples = self.ripples.get();
        if ripples.is_empty()
            && self
                .tiles
                .get()
                .iter()
                .all(|t| t.panels.len() == 1 && t.panels[0].progress >= 1.0)
        {
            self.play_logo();
        }
        let id = self.next_ripple_id.get();
        ripples.push(Ripple::new(
            id,
            x * width,
            y * height,
            width,
            height,
            elapsed_millis(ctx),
        ));
        self.ripples.set(ripples);
        self.next_ripple_id.set(id + 1);
        self.handle_pre_render(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let compact = width < 720.0;
        self.is_compact.set_if_neq(compact);
        let now = elapsed_millis(ctx);
        let mut ripples = self.ripples.get();
        let mut tiles = self.tiles.get();
        let was_active = !ripples.is_empty() || tiles.iter().any(|t| t.panels.len() > 1);
        if !was_active {
            return;
        }
        let columns = if compact { 3 } else { 5 };
        for ripple in &mut ripples {
            ripple.update(now, width, height);
            for tile in &mut tiles {
                let bit = 1u32 << tile.id;
                if ripple.hits & bit == 0 && ripple.touches_tile(tile.id, columns, width, height) {
                    ripple.hits |= bit;
                    tile.panels
                        .insert(0, Panel::new(ripple.id, rand::random(), now, false));
                }
            }
        }
        for tile in &mut tiles {
            tile.advance(now);
        }
        ripples.retain(|r| r.progress < 1.0);
        self.ripples.set(ripples);
        self.tiles.set(tiles);
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
}

fn elapsed_millis(ctx: &NodeContext) -> u64 {
    ctx.elapsed_time_millis().min(u64::MAX as u128) as u64
}

fn normalized_point(ctx: &NodeContext, x: f64, y: f64) -> (f64, f64) {
    let p = ctx.local_point(Point2::new(x, y));
    (p.x.clamp(0.0, 1.0), p.y.clamp(0.0, 1.0))
}
