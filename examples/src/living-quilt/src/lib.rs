use pax_kit::math::Point2;
use pax_kit::*;
use rand::Rng;

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

const MAX_LIVE_RIPPLES: usize = 5;
const IDLE_TILE_MS: u64 = 500;

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
    // Rings and idle panel changes share an ID sequence so repeat keys stay unique.
    pub next_animation_id: Property<usize>,
    pub idle_active: Property<bool>,
    pub next_idle_tile_ms: Property<u64>,
    pub next_auto_ring_ms: Property<u64>,
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
        self.next_animation_id.set(1);
        self.idle_active.set(false);
        self.next_idle_tile_ms.set(0);
        self.schedule_auto_ring(elapsed_millis(ctx));
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
        self.try_emit_ripple(x, y, width, height, elapsed_millis(ctx));
    }

    pub fn handle_logo_click(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        // Replay is unconditional. The click also bubbles to the canvas for
        // one ring attempt, whose concurrency cap does not gate the logo.
        self.play_logo();
    }

    fn try_emit_ripple(&self, x: f64, y: f64, width: f64, height: f64, now: u64) -> bool {
        // Admission follows normal removal, not just elapsed time: rejected
        // ring attempts queue nothing and leave accepted waves untouched.
        if self.ripples.read(Vec::len) >= MAX_LIVE_RIPPLES {
            return false;
        }
        self.light_x
            .ease_to(x, Duration::Milliseconds(420.into()), EasingCurve::OutQuad);
        self.light_y
            .ease_to(y, Duration::Milliseconds(420.into()), EasingCurve::OutQuad);
        self.idle_active.set(false);
        self.next_idle_tile_ms.set(0);
        self.schedule_auto_ring(now);
        let id = self.next_animation_id.get();
        self.push_ripple(Ripple::new(id, x * width, y * height, width, height, now));
        self.next_animation_id.set(id + 1);
        self.advance_scene(now, width, height);
        true
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        let now = elapsed_millis(ctx);
        self.advance_scene(now, width, height);
        self.advance_idle(now, width, height);
    }

    fn idle_can_run(&self, now: u64) -> bool {
        self.logo_progress.get() >= 0.999
            && self.ripples.read(Vec::is_empty)
            && (self.idle_active.get()
                || self.tiles.read(|tiles| {
                    tiles
                        .iter()
                        .all(|t| t.panels.len() == 1 && t.panels[0].progress(now) >= 1.0)
                }))
    }

    fn schedule_auto_ring(&self, now: u64) {
        self.next_auto_ring_ms
            .set(now.saturating_add(rand::thread_rng().gen_range(3000..=5000)));
    }

    fn advance_idle(&self, now: u64, width: f64, height: f64) {
        if width <= 0.0 || height <= 0.0 || self.tiles.read(Vec::is_empty) {
            return;
        }
        if !self.idle_can_run(now) {
            return;
        }
        if !self.idle_active.get() {
            // Ambient slides last longer than their cadence. Latch idle until
            // a ring is accepted, rather than letting those slides disarm it.
            self.idle_active.set(true);
            self.next_idle_tile_ms.set(now.saturating_add(IDLE_TILE_MS));
        }
        if self.next_auto_ring_ms.get() == 0 {
            self.schedule_auto_ring(now);
        }
        if now >= self.next_auto_ring_ms.get() {
            let mut rng = rand::thread_rng();
            self.try_emit_ripple(rng.gen(), rng.gen(), width, height, now);
            return;
        }
        if now < self.next_idle_tile_ms.get() {
            return;
        }

        let mut rng = rand::thread_rng();
        let count = rng.gen_range(3..=9).min(self.tiles.read(Vec::len));
        let selected = rand::seq::index::sample(&mut rng, self.tiles.read(Vec::len), count);
        let id = self.next_animation_id.get();
        self.tiles.update(|tiles| {
            for index in selected.iter() {
                tiles[index].push_panel(id, rng.gen(), rng.gen_range(0..4), now);
            }
        });
        self.next_animation_id.set(id + 1);
        // No catch-up queue after suspension or a slow frame.
        self.next_idle_tile_ms.set(now.saturating_add(IDLE_TILE_MS));
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
    fn ring_repeat_changes_only_at_birth_and_removal() {
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
        for id in 0..MAX_LIVE_RIPPLES {
            example.push_ripple(Ripple::new(id, 50.0, 50.0, 1000.0, 600.0, id as u64 * 50));
        }
        binding.get();
        assert_eq!(example.ripples.read(Vec::len), MAX_LIVE_RIPPLES);
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
        example
            .ripple_hits
            .set((0..MAX_LIVE_RIPPLES as u32).collect());
        example.advance_scene(1250, 1000.0, 600.0);
        binding.get();
        assert_eq!(
            example
                .ripples
                .read(|rings| rings.iter().map(|r| r.id).collect::<Vec<_>>()),
            (2..MAX_LIVE_RIPPLES).collect::<Vec<_>>()
        );
        assert_eq!(
            example.ripple_hits.get(),
            (2..MAX_LIVE_RIPPLES as u32).collect::<Vec<_>>()
        );
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

    fn populated_example() -> Example {
        Example {
            logo_progress: Property::new(1.0),
            light_x: Property::new(0.5),
            light_y: Property::new(0.5),
            next_animation_id: Property::new(1),
            tiles: Property::new(
                (0..15)
                    .map(|id| TileState {
                        id,
                        panels: vec![Panel::new(0, id as u64, 0, true)],
                    })
                    .collect(),
            ),
            ..Default::default()
        }
    }

    #[test]
    fn idle_changes_three_to_nine_distinct_tiles_every_half_second() {
        let example = populated_example();
        example.next_auto_ring_ms.set(60000);
        example.advance_idle(0, 1000.0, 600.0);
        assert!(example.idle_active.get());
        example.advance_idle(499, 1000.0, 600.0);
        assert_eq!(example.next_animation_id.get(), 1);

        for batch in 1..=20 {
            let now = batch as u64 * IDLE_TILE_MS;
            example.advance_scene(now, 1000.0, 600.0);
            example.advance_idle(now, 1000.0, 600.0);
            example.tiles.read(|tiles| {
                let changed: Vec<_> = tiles.iter().filter(|t| t.panels[0].id == batch).collect();
                assert!((3..=9).contains(&changed.len()));
                for tile in changed {
                    assert_eq!(tile.panels[0].started_at_ms, now);
                    assert!(tile.panels[0].direction < 4);
                    assert!(!tile.panels[0].initially_settled);
                    let ids: std::collections::HashSet<_> =
                        tile.panels.iter().map(|p| p.id).collect();
                    assert_eq!(ids.len(), tile.panels.len());
                }
            });
            assert!(
                example.idle_can_run(now),
                "ambient slides must not stop the idle cadence"
            );
            assert!(example.ripples.read(Vec::is_empty));
            assert_eq!(example.logo_progress.get(), 1.0);
            assert_eq!((example.light_x.get(), example.light_y.get()), (0.5, 0.5));
            assert_eq!(example.next_animation_id.get(), batch + 1);
            example.advance_idle(now + 499, 1000.0, 600.0);
            assert_eq!(example.next_animation_id.get(), batch + 1);
        }
    }

    #[test]
    fn idle_waits_for_logo_and_ring_driven_panels_to_settle() {
        let example = populated_example();
        example.next_auto_ring_ms.set(9000);
        example.logo_progress.set(0.8);
        example.advance_idle(100, 1000.0, 600.0);
        assert!(!example.idle_active.get());
        example.logo_progress.set(1.0);
        example
            .tiles
            .update(|tiles| tiles[0].push_panel(1, 10, 2, 100));
        example.advance_scene(500, 1000.0, 600.0);
        example.advance_idle(500, 1000.0, 600.0);
        assert!(!example.idle_active.get());
        example.advance_scene(680, 1000.0, 600.0);
        example.advance_idle(680, 1000.0, 600.0);
        assert!(example.idle_active.get());
        assert_eq!(example.next_idle_tile_ms.get(), 1180);
    }

    #[test]
    fn canvas_click_resets_idle_deadlines_without_replaying_logo() {
        let frames = Property::new(0_u64);
        let millis = Property::new(0_u64);
        register_time(&frames);
        register_millis(&millis);
        let example = populated_example();
        example.next_auto_ring_ms.set(600);
        example.advance_idle(0, 1000.0, 600.0);
        millis.set(500);
        example.advance_idle(500, 1000.0, 600.0);
        assert!(example
            .tiles
            .read(|tiles| tiles.iter().any(|t| t.panels.len() > 1)));
        assert!(example.try_emit_ripple(0.2, 0.8, 1000.0, 600.0, 500));
        assert!(!example.idle_active.get());
        assert_eq!(example.next_idle_tile_ms.get(), 0);
        assert!((3500..=5500).contains(&example.next_auto_ring_ms.get()));
        assert_eq!(example.logo_progress.get(), 1.0);
        let next_id = example.next_animation_id.get();
        millis.set(600);
        example.advance_idle(600, 1000.0, 600.0);
        assert_eq!(example.next_animation_id.get(), next_id);
        assert_eq!(example.ripples.read(Vec::len), 1);
    }

    #[test]
    fn logo_replay_restarts_mid_animation_even_when_ring_capacity_is_full() {
        let frames = Property::new(0_u64);
        let millis = Property::new(0_u64);
        register_time(&frames);
        register_millis(&millis);
        let example = populated_example();
        for _ in 0..MAX_LIVE_RIPPLES {
            assert!(example.try_emit_ripple(0.5, 0.5, 1000.0, 600.0, 0));
        }
        assert_eq!(example.logo_progress.get(), 1.0);
        for now in [0, 150, 400, 700] {
            millis.set(now);
            // The card handler replays first; bubbling then attempts a ring.
            assert!(example.logo_progress.get() > 0.0);
            example.play_logo();
            assert_eq!(example.logo_progress.get(), 0.0);
            assert!(!example.try_emit_ripple(0.5, 0.5, 1000.0, 600.0, now));
            assert_eq!(example.logo_progress.get(), 0.0);
            assert_eq!(example.ripples.read(Vec::len), MAX_LIVE_RIPPLES);
        }
        millis.set(2140);
        assert_eq!(example.logo_progress.get(), 1.0);
    }

    #[test]
    fn ring_emission_does_not_restart_an_in_progress_logo_replay() {
        let frames = Property::new(0_u64);
        let millis = Property::new(0_u64);
        register_time(&frames);
        register_millis(&millis);
        let example = populated_example();
        example.play_logo();
        for now in [100, 200, 300] {
            millis.set(now);
            let progress = example.logo_progress.get();
            assert!(progress > 0.0 && progress < 1.0);
            assert!(example.try_emit_ripple(0.1, 0.8, 1000.0, 600.0, now));
            assert_eq!(example.logo_progress.get(), progress);
        }
        millis.set(1440);
        assert_eq!(example.logo_progress.get(), 1.0);
    }

    #[test]
    fn automatic_ring_moves_light_without_replaying_logo_and_resamples_delay() {
        let frames = Property::new(0_u64);
        let millis = Property::new(0_u64);
        register_time(&frames);
        register_millis(&millis);
        let example = populated_example();
        example.schedule_auto_ring(0);
        let due = example.next_auto_ring_ms.get();
        assert!((3000..=5000).contains(&due));
        example.advance_idle(0, 1000.0, 600.0);
        millis.set(due - 1);
        example.advance_idle(due - 1, 1000.0, 600.0);
        assert!(example.ripples.read(Vec::is_empty));
        let idle_panel_id = example.next_animation_id.get() - 1;
        millis.set(due);
        example.advance_idle(due, 1000.0, 600.0);
        let ring = example.ripples.get().remove(0);
        assert_eq!(ring.id, idle_panel_id + 1);
        assert_eq!(ring.started_at_ms, due);
        assert!((0.0..1.0).contains(&ring.x) && (0.0..1.0).contains(&ring.y));
        assert!(!example.idle_active.get());
        assert_eq!(example.logo_progress.get(), 1.0);
        assert!((due + 3000..=due + 5000).contains(&example.next_auto_ring_ms.get()));
        millis.set(due + 420);
        assert!((example.light_x.get() - ring.x).abs() < 1e-12);
        assert!((example.light_y.get() - ring.y).abs() < 1e-12);
        for offset in (420..=2000).step_by(20) {
            millis.set(due + offset);
            example.advance_scene(due + offset, 1000.0, 600.0);
        }
        millis.set(due + 2000);
        example.advance_idle(due + 2000, 1000.0, 600.0);
        assert!(example.idle_active.get());
        assert_eq!(example.next_idle_tile_ms.get(), due + 2500);
    }

    #[test]
    fn idle_does_not_queue_catch_up_work_or_animate_zero_sized_scenes() {
        let frames = Property::new(0_u64);
        let millis = Property::new(0_u64);
        register_time(&frames);
        register_millis(&millis);
        let example = populated_example();
        example.next_auto_ring_ms.set(60000);
        example.advance_idle(0, 0.0, 600.0);
        assert!(!example.idle_active.get());
        example.advance_idle(0, 1000.0, 600.0);
        example.advance_idle(10000, 1000.0, 600.0);
        assert_eq!(
            example.next_animation_id.get(),
            2,
            "only one tile batch after a long pause"
        );
        assert_eq!(example.next_idle_tile_ms.get(), 10500);
        millis.set(100000);
        example.advance_idle(100000, 1000.0, 600.0);
        assert_eq!(
            example.ripples.read(Vec::len),
            1,
            "only one overdue automatic ring"
        );
        assert_eq!(example.next_animation_id.get(), 3);
        assert!((103000..=105000).contains(&example.next_auto_ring_ms.get()));
    }

    #[test]
    fn unattended_cycles_remain_bounded_with_unique_panel_keys() {
        let frames = Property::new(0_u64);
        let millis = Property::new(0_u64);
        register_time(&frames);
        register_millis(&millis);
        let example = populated_example();
        let mut last_ring = 0;
        let mut last_start = 0;
        let mut emitted = 0;
        for now in (0..60000).step_by(20) {
            millis.set(now);
            example.advance_scene(now, 390.0, 844.0);
            example.advance_idle(now, 390.0, 844.0);
            example.ripples.read(|rings| {
                assert!(
                    rings.len() <= 1,
                    "automatic rings should settle between pulses"
                );
                if let Some(ring) = rings.first() {
                    if ring.id != last_ring {
                        assert!((3000..=5020).contains(&(ring.started_at_ms - last_start)));
                        last_ring = ring.id;
                        last_start = ring.started_at_ms;
                        emitted += 1;
                    }
                }
            });
            example.tiles.read(|tiles| {
                for tile in tiles {
                    assert!(tile.panels.len() <= 4, "completed panels must retire");
                    let ids: std::collections::HashSet<_> =
                        tile.panels.iter().map(|p| p.id).collect();
                    assert_eq!(ids.len(), tile.panels.len());
                }
            });
        }
        assert!((11..=19).contains(&emitted));
    }

    #[test]
    fn full_ring_cap_ignores_clicks_until_normal_removal() {
        let frames = Property::new(0_u64);
        let millis = Property::new(0_u64);
        register_time(&frames);
        register_millis(&millis);
        let example = populated_example();
        for i in 0..MAX_LIVE_RIPPLES {
            let now = i as u64 * 50;
            millis.set(now);
            assert!(example.try_emit_ripple(0.7, 0.4, 1000.0, 600.0, now));
        }
        let before = example.ripples.get().to_pax_value();
        let hits = example.ripple_hits.get();
        let tiles = example.tiles.get().to_pax_value();
        let light = (example.light_x.get(), example.light_y.get());
        let logo = example.logo_progress.get();
        let clock = example.wave_time.get();
        let idle_deadline = example.next_auto_ring_ms.get();
        for now in [201, 500, WAVE_MS as u64 + 1] {
            assert!(!example.try_emit_ripple(0.1, 0.9, 1000.0, 600.0, now));
            assert_eq!(example.ripples.get().to_pax_value(), before);
            assert_eq!(example.ripple_hits.get(), hits);
            assert_eq!(example.tiles.get().to_pax_value(), tiles);
            assert_eq!((example.light_x.get(), example.light_y.get()), light);
            assert_eq!(example.logo_progress.get(), logo);
            assert_eq!(example.wave_time.get(), clock);
            assert_eq!(example.next_auto_ring_ms.get(), idle_deadline);
            assert_eq!(example.next_animation_id.get(), 6);
        }
        millis.set(1000);
        assert_eq!((example.light_x.get(), example.light_y.get()), (0.7, 0.4));
        // Pointer movement remains independent of ring admission.
        example.follow_mouse_light(0.9, 0.1);
        assert_eq!((example.light_x.get(), example.light_y.get()), (0.9, 0.1));

        millis.set(WAVE_MS as u64);
        example.advance_scene(WAVE_MS as u64, 1000.0, 600.0);
        assert_eq!(
            example
                .ripples
                .read(|r| r.iter().map(|r| r.id).collect::<Vec<_>>()),
            vec![2, 3, 4, 5]
        );
        assert!(example.try_emit_ripple(0.1, 0.9, 1000.0, 600.0, WAVE_MS as u64));
        assert_eq!(
            example
                .ripples
                .read(|r| r.iter().map(|r| r.id).collect::<Vec<_>>()),
            vec![2, 3, 4, 5, 6]
        );
        assert_eq!(example.ripple_hits.read(Vec::len), MAX_LIVE_RIPPLES);
        assert_eq!(example.next_animation_id.get(), 7);
    }

    #[test]
    fn sustained_clicks_stay_capped_and_all_accepted_work_settles() {
        let frames = Property::new(0_u64);
        let millis = Property::new(0_u64);
        register_time(&frames);
        register_millis(&millis);
        let example = populated_example();
        let mut accepted = 0;
        let mut rejected = 0;
        for now in (0..6000_u64).step_by(20) {
            millis.set(now);
            example.advance_scene(now, 1000.0, 600.0);
            if now % 60 == 0 {
                let phase = now as f64 / 6000.0;
                if example.try_emit_ripple(phase, 1.0 - phase, 1000.0, 600.0, now) {
                    accepted += 1;
                } else {
                    rejected += 1;
                }
            }
            assert!(example.ripples.read(Vec::len) <= MAX_LIVE_RIPPLES);
            assert_eq!(
                example.ripples.read(Vec::len),
                example.ripple_hits.read(Vec::len)
            );
        }
        assert!(
            accepted > MAX_LIVE_RIPPLES,
            "capacity must reopen during sustained input"
        );
        assert!(rejected > 0);
        assert_eq!(example.next_animation_id.get(), accepted + 1);
        for now in (6000..8000_u64).step_by(20) {
            millis.set(now);
            example.advance_scene(now, 1000.0, 600.0);
        }
        assert!(example.ripples.read(Vec::is_empty));
        assert!(example.ripple_hits.read(Vec::is_empty));
        assert!(example.tiles.read(|tiles| tiles
            .iter()
            .all(|t| t.panels.len() == 1 && t.panels[0].progress(8000) == 1.0)));
        let clock = example.wave_time.get();
        example.advance_scene(10000, 1000.0, 600.0);
        assert_eq!(
            example.wave_time.get(),
            clock,
            "ignored input must leave no queued work"
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
