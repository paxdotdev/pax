use pax_kit::*;

/// Bounded, grayscale demonstrations for the feature rail. Each instance mounts
/// only its selected study and advances only while its artboard is visible.
#[pax]
#[file("feature_gallery/studies.pax")]
pub struct FeatureStudy {
    pub kind: Property<String>,
    pub progress: Property<f64>,
    pub alternate: Property<bool>,
    pub present: Property<bool>,
    pub phase: Property<f64>,
    pub items: Property<Vec<usize>>,
    pub quantity: Property<usize>,
    pub total: Property<usize>,
    pub _visible: Property<bool>,
    pub _last_ms: Property<f64>,
}

impl FeatureStudy {
    pub fn mount(&mut self, ctx: &NodeContext) {
        if self.kind.get() == "computed" {
            let quantity = self.quantity.clone();
            let dependencies = [quantity.untyped()];
            self.total.replace_with(Property::computed(
                move || 12 * quantity.get(),
                &dependencies,
            ));
        }
        self._last_ms.set(ctx.elapsed_time_millis() as f64);
        self.phase.set(1.5);
        self.update_picture();
    }

    pub fn visibility(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self._visible.set_if_neq(event.current.is_in_viewport());
        self._last_ms.set(ctx.elapsed_time_millis() as f64);
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        if !self._visible.get() || self.kind.get() == "caps" {
            return;
        }
        let now = ctx.elapsed_time_millis() as f64;
        let dt = ((now - self._last_ms.get()) / 1000.0).clamp(0.0, 0.05);
        self._last_ms.set(now);
        self.phase.set((self.phase.get() + dt).rem_euclid(8.0));
        self.update_picture();
    }

    fn update_picture(&mut self) {
        let phase = self.phase.get();
        self.progress.set_if_neq(study_progress(phase));
        let alternate = phase >= 4.0;
        self.alternate.set_if_neq(alternate);
        if self.kind.get() == "computed" {
            self.quantity.set_if_neq(if alternate { 3 } else { 2 });
        }
        // The interrupted study reverses before the one-second exit completes.
        self.present.set_if_neq(if self.kind.get() == "interrupt" {
            !(3.0..3.4).contains(&phase) && !(5.0..6.3).contains(&phase)
        } else {
            phase < 4.0
        });
        if self.kind.get() == "reflow" {
            self.items
                .set_if_neq(if alternate { vec![0, 2] } else { vec![0, 1, 2] });
        }
    }
}

// A legible hold at either end, with a smooth round trip and no reset jump.
fn study_progress(phase: f64) -> f64 {
    let t = if phase < 4.0 {
        (phase - 0.8) / 2.4
    } else {
        (7.2 - phase) / 2.4
    }
    .clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_authored_study_has_a_rendering_branch() {
        let gallery = include_str!("gallery.pax");
        let studies = include_str!("studies.pax");
        let mut count = 0;
        for tail in gallery.split("kind=\"").skip(1) {
            let kind = tail.split('"').next().unwrap();
            assert!(
                studies.contains(&format!("self.kind == \"{kind}\"")),
                "{kind}"
            );
            count += 1;
        }
        assert_eq!(count, 28);
        assert_eq!(gallery.matches("<FeatureCard ").count(), 43);
        assert!(!gallery.contains("&amp;"));
        assert!(!gallery.contains("SiteTheme::cyan"));
    }

    #[test]
    fn study_clock_is_bounded_and_holds_at_each_end() {
        for i in 0..=800 {
            assert!((0.0..=1.0).contains(&study_progress(i as f64 / 100.0)));
        }
        assert_eq!(study_progress(0.0), study_progress(8.0));
        assert_eq!(study_progress(0.8), 0.0);
        assert_eq!(study_progress(3.2), 1.0);
        assert_eq!(study_progress(4.8), 1.0);
        assert_eq!(study_progress(7.2), 0.0);
    }
}
