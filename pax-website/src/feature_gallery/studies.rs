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
            let quantity = self.quantity.local();
            let dependencies = [quantity.untyped()];
            self.total.replace_with(LocalProperty::computed(
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
        self.phase
            .set((self.phase.get() + dt).rem_euclid(study_period(&self.kind.get())));
        self.update_picture();
    }

    fn update_picture(&mut self) {
        let phase = self.phase.get();
        self.progress.set_if_neq(study_progress(phase));
        let alternate = phase >= study_period(&self.kind.get()) / 2.0;
        self.alternate.set_if_neq(alternate);
        if self.kind.get() == "computed" {
            self.quantity.set_if_neq(if alternate { 3 } else { 2 });
        }
        // The interrupted study reverses before the one-second exit completes.
        self.present.set_if_neq(if self.kind.get() == "interrupt" {
            !(2.0..2.4).contains(&phase) && !(3.7..4.85).contains(&phase)
        } else {
            !alternate
        });
        if self.kind.get() == "reflow" {
            self.items
                .set_if_neq(if alternate { vec![0, 2] } else { vec![0, 1, 2] });
        }
    }
}

// Halve idle holds, not the authored motion durations. Discrete illustrations
// have no transition to preserve; enter/reflow keep their 1s/0.5s transitions.
fn study_period(kind: &str) -> f64 {
    match kind {
        "theme" | "classes" | "autosize" | "computed" => 4.0,
        "enter" => 5.0,
        "reflow" => 4.5,
        "interrupt" => 6.2,
        _ => 6.4,
    }
}

// The 2.4s strokes are unchanged; each 1.6s endpoint hold is now 0.8s.
// The zero hold is split across the loop boundary so there is no reset jump.
fn study_progress(phase: f64) -> f64 {
    let t = if phase < 3.2 {
        (phase - 0.4) / 2.4
    } else {
        (6.0 - phase) / 2.4
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
        for i in 0..=640 {
            assert!((0.0..=1.0).contains(&study_progress(i as f64 / 100.0)));
        }
        assert_eq!(study_progress(0.0), study_progress(6.4));
        assert_eq!(study_progress(0.4), 0.0);
        assert_eq!(study_progress(2.8), 1.0);
        assert_eq!(study_progress(3.6), 1.0);
        assert_eq!(study_progress(6.0), 0.0);
        assert!((study_progress(1.6) - 0.5).abs() < 1e-9);
        assert!((study_progress(4.8) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn discrete_and_transition_studies_halve_only_their_idle_intervals() {
        assert_eq!(study_period("autosize") / 2.0, 4.0 / 2.0);
        assert_eq!(study_period("theme"), study_period("computed"));
        assert_eq!(study_period("enter") / 2.0 - 1.0, (4.0 - 1.0) / 2.0);
        assert_eq!(study_period("reflow") / 2.0 - 0.5, (4.0 - 0.5) / 2.0);
        let mut study = FeatureStudy::default();
        study.kind.set("interrupt".to_string());
        for (phase, present) in [
            (1.9, true),
            (2.1, false),
            (2.5, true),
            (3.8, false),
            (4.8, false),
            (4.9, true),
        ] {
            study.phase.set(phase);
            study.update_picture();
            assert_eq!(study.present.get(), present);
        }
    }
}
