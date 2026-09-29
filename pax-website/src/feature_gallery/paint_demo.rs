use pax_kit::*;

/// Small live paint studies. The visible card owns the clock, not its mask source.
#[pax]
#[file("feature_gallery/paint_demo.pax")]
pub struct FeaturePaintDemo {
    pub mask: Property<bool>,
    pub progress: Property<f64>,
    pub band_x: Property<f64>,
    pub _visible: Property<bool>,
    pub _last_ms: Property<f64>,
    pub _phase: Property<f64>,
}

impl FeaturePaintDemo {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self._last_ms.set(ctx.elapsed_time_millis() as f64);
        // A readable first frame even before the first viewport notification.
        self._phase.set(1.8);
        self.update_picture();
    }

    pub fn visibility(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self._visible.set_if_neq(event.current.is_in_viewport());
        self._last_ms.set(ctx.elapsed_time_millis() as f64);
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        if !self._visible.get() {
            return;
        }
        let now = ctx.elapsed_time_millis() as f64;
        let dt = ((now - self._last_ms.get()) / 1000.0).clamp(0.0, 0.05);
        self._last_ms.set(now);
        if dt > 0.0 {
            self._phase.set((self._phase.get() + dt).rem_euclid(7.0));
            self.update_picture();
        }
    }

    fn update_picture(&mut self) {
        self.progress.set_if_neq(reveal_progress(self._phase.get()));
        self.band_x.set_if_neq(-100.0 * self._phase.get() / 7.0);
    }
}

// Draw, rest long enough to read, then gently rewind instead of flashing blank.
fn reveal_progress(phase: f64) -> f64 {
    let t = if phase < 2.8 {
        phase / 2.8
    } else if phase < 5.6 {
        1.0
    } else {
        (7.0 - phase) / 1.4
    }
    .clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveal_holds_and_returns_continuously() {
        assert_eq!(reveal_progress(0.0), 0.0);
        assert_eq!(reveal_progress(2.8), 1.0);
        assert_eq!(reveal_progress(5.6), 1.0);
        assert_eq!(reveal_progress(7.0), 0.0);
        for step in 0..=700 {
            assert!((0.0..=1.0).contains(&reveal_progress(step as f64 / 100.0)));
        }
        assert!((reveal_progress(6.999) - reveal_progress(0.001)).abs() < 0.00001);
    }

    #[test]
    fn new_cards_follow_the_lead_pair_and_link_to_canonical_docs() {
        let gallery = include_str!("gallery.pax");
        let titles = [
            "Rust Hot Reload",
            "Path Drawing Animations",
            "Layered Paint",
            "Dynamic Vector Masks",
        ];
        let positions: Vec<_> = titles
            .iter()
            .map(|title| {
                gallery
                    .find(&format!("title=\"{title}\""))
                    .expect("card title exists")
            })
            .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        for (url, article, heading) in [
            (
                "drawing-styling.html#paint-layers",
                include_str!("../../../pax-docs/book/src/drawing-styling.md"),
                "## Paint layers",
            ),
            (
                "compositing-effects.html#painted-alpha-masks",
                include_str!("../../../pax-docs/book/src/compositing-effects.md"),
                "### Painted alpha masks",
            ),
        ] {
            assert!(gallery.contains(&format!("https://docs.pax.dev/{url}")));
            assert!(article.contains(heading));
        }
    }
}
