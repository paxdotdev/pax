use pax_kit::math::Point2;
use pax_kit::*;

pub mod animated_pax_logo;
pub mod animated_pax_logo_banner;
pub mod animated_pax_logo_post;
pub mod logo_card;
pub mod quilt_layer;
pub mod quilt_panel;
pub mod quilt_scene;
pub mod quilt_tile;
pub mod static_pax_logo;
pub mod wave;
pub use animated_pax_logo::*;
pub use animated_pax_logo_banner::*;
pub use animated_pax_logo_post::*;
pub use logo_card::*;
pub use quilt_layer::*;
pub use quilt_panel::*;
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
    // Contact bookkeeping is Rust-only; updating it must not republish the repeat.
    pub ripple_hits: Property<Vec<u32>>,
    pub wave_time: Property<u64>,
    pub wave_width: Property<f64>,
    pub wave_height: Property<f64>,
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
        self.follow_mouse_light(x, y);
    }

    fn follow_mouse_light(&self, x: f64, y: f64) {
        // Direct pointer input takes over from the click/touch tween.
        self.light_x.cancel_transitions();
        self.light_y.cancel_transitions();
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
        if self.ripples.read(|ripples| ripples.is_empty())
            && self.tiles.read(|tiles| {
                tiles.iter().all(|t| {
                    t.panels.len() == 1 && t.panels[0].progress(elapsed_millis(ctx)) >= 1.0
                })
            })
        {
            self.play_logo();
        }
        let id = self.next_ripple_id.get();
        self.push_ripple(Ripple::new(
            id,
            x * width,
            y * height,
            width,
            height,
            elapsed_millis(ctx),
        ));
        self.next_ripple_id.set(id + 1);
        self.handle_pre_render(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        self.advance_scene(elapsed_millis(ctx), width, height);
    }

    fn push_ripple(&self, ripple: Ripple) {
        self.ripples.update(|ripples| ripples.push(ripple));
        self.ripple_hits.update(|hits| hits.push(0));
    }

    fn advance_scene(&self, now: u64, width: f64, height: f64) {
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let compact = width < 720.0;
        self.is_compact.set_if_neq(compact);
        let was_active = self.ripples.read(|ripples| !ripples.is_empty())
            || self
                .tiles
                .read(|tiles| tiles.iter().any(|t| t.panels.len() > 1));
        if !was_active {
            return;
        }
        let columns = if compact { 3 } else { 5 };
        let mut arrivals = Vec::new();
        let mut hits = self.ripple_hits.get();
        self.ripples.read(|ripples| {
            for (ripple, hits) in ripples.iter().zip(&mut hits) {
                let sample = ripple.sample(now, width, height);
                self.tiles.read(|tiles| {
                    for tile in tiles {
                        let bit = 1u32 << tile.id;
                        if *hits & bit == 0 {
                            if let Some(direction) =
                                sample.contact_direction(tile.id, columns, width, height)
                            {
                                *hits |= bit;
                                arrivals.push((tile.id, ripple.id, direction));
                            }
                        }
                    }
                });
            }
        });
        // Structural updates happen only at arrivals/retirements. Each mounted
        // panel advances its own small motion properties between those events.
        if !arrivals.is_empty()
            || self
                .tiles
                .read(|tiles| tiles.iter().any(|t| t.needs_retirement(now)))
        {
            let mut tiles = self.tiles.get();
            for (tile_id, ripple_id, direction) in arrivals {
                tiles[tile_id].push_panel(ripple_id, rand::random(), direction, now);
            }
            for tile in &mut tiles {
                tile.advance(now);
            }
            self.tiles.set(tiles);
        }
        // The repeat changes only at birth/removal. Clock and bounds invalidate
        // individual ring paths without reconverting the whole ring collection.
        if self
            .ripples
            .read(|ripples| ripples.iter().any(|r| r.is_finished(now)))
        {
            let mut ripples = self.ripples.get();
            let mut index = 0;
            hits.retain(|_| {
                let keep = !ripples[index].is_finished(now);
                index += 1;
                keep
            });
            ripples.retain(|r| !r.is_finished(now));
            self.ripples.set(ripples);
        }
        self.ripple_hits.set_if_neq(hits);
        self.wave_time.set_if_neq(now);
        self.wave_width.set_if_neq(width);
        self.wave_height.set_if_neq(height);
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

#[cfg(test)]
mod tests {
    use super::*;
    use pax_kit::properties::{register_millis, register_time};
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn ring_repeat_changes_only_at_birth_and_removal_without_a_concurrency_cap() {
        let example = Example::default();
        let source = Variable::new_from_typed_property(example.ripples.clone());
        let source_reads = Rc::new(Cell::new(0));
        let reads = source_reads.clone();
        let dependency = source.get_untyped_property().clone();
        let binding = Property::computed(
            move || {
                reads.set(reads.get() + 1);
                source.get_as_pax_value()
            },
            &[dependency],
        );
        for id in 0..8 {
            example.push_ripple(Ripple::new(id, 50.0, 50.0, 1000.0, 600.0, id as u64 * 50));
        }
        binding.get();
        assert_eq!(example.ripples.read(Vec::len), 8);
        for now in [400, 600, 800, 1000] {
            example.advance_scene(now, 1000.0, 600.0);
            binding.get();
        }
        assert_eq!(
            source_reads.get(),
            1,
            "frame updates must not invalidate repeat data"
        );

        // Preserve keyed siblings and their hit bookkeeping when only some expire.
        example.ripple_hits.set((0..8).collect());
        example.advance_scene(1250, 1000.0, 600.0);
        binding.get();
        assert_eq!(
            example
                .ripples
                .read(|rings| rings.iter().map(|r| r.id).collect::<Vec<_>>()),
            (2..8).collect::<Vec<_>>()
        );
        assert_eq!(example.ripple_hits.get(), (2..8).collect::<Vec<_>>());
        assert_eq!(source_reads.get(), 2);
        example.advance_scene(1600, 1000.0, 600.0);
        binding.get();
        assert!(example.ripples.read(Vec::is_empty));
        assert!(example.ripple_hits.read(Vec::is_empty));
        assert_eq!(source_reads.get(), 3);
        example.advance_scene(2000, 1000.0, 600.0);
        assert_eq!(
            example.wave_time.get(),
            1600,
            "idle scenes stop publishing the wave clock"
        );
    }

    #[test]
    fn mouse_light_takes_over_without_the_click_tween_writing_back() {
        let frames = Property::new(0_u64);
        let millis = Property::new(0_u64);
        register_time(&frames);
        register_millis(&millis);
        let example = Example {
            light_x: Property::new(0.5),
            light_y: Property::new(0.5),
            ..Default::default()
        };
        example.light_x.ease_to(
            0.25,
            Duration::Milliseconds(420.into()),
            EasingCurve::OutQuad,
        );
        example.light_y.ease_to(
            0.75,
            Duration::Milliseconds(420.into()),
            EasingCurve::OutQuad,
        );
        millis.set(84);
        assert!((0.25..0.5).contains(&example.light_x.get()));
        assert!((0.5..0.75).contains(&example.light_y.get()));

        example.follow_mouse_light(0.9, 0.1);
        for now in [84, 100, 210, 420, 840] {
            millis.set(now);
            assert_eq!((example.light_x.get(), example.light_y.get()), (0.9, 0.1));
        }
    }
}
