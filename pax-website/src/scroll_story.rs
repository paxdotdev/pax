/// Stable chapter geometry and local clocks. Animated children never feed back
/// into these extents; changing scroll direction simply samples an earlier pose.
pub struct OpeningStory {
    pub hero_span: f64,
    pub runtime_span: f64,
    pub hero_exit: f64,
    pub runtime_progress: f64,
}

impl OpeningStory {
    pub fn sample(scroll: f64, viewport: (f64, f64), hero: f64, runtime: f64) -> Self {
        let height = viewport.1.max(1.0);
        let compact = viewport.0 < 760.0;
        // A tall screen must not turn a minimum scene duration into a blank
        // viewport. Cap extra breathing room until a scene can occupy it.
        let hero_span = hero.max(580.0).max(height.min(hero + 96.0));
        let runtime_span = runtime
            .max(790.0)
            .max((height * if compact { 1.0 } else { 2.0 }).min(runtime + 160.0));
        let scroll = scroll.max(0.0);
        Self {
            hero_span,
            runtime_span,
            hero_exit: (scroll / hero_span).clamp(0.0, 1.0) * 100.0,
            runtime_progress: section_progress(scroll, height, hero_span, runtime_span),
        }
    }
}

/// Map a chapter's approach and departure to a local 0–100 timeline. This is
/// stateless: jumping, reversing, or resizing cannot leave a stale animation.
pub fn section_progress(scroll: f64, viewport_height: f64, top: f64, height: f64) -> f64 {
    let viewport_height = viewport_height.max(1.0);
    let start = top - viewport_height * 0.90;
    let end = top + height.max(1.0) - viewport_height * 0.30;
    ((scroll.max(0.0) - start) / (end - start)).clamp(0.0, 1.0) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hero_is_complete_at_the_top() {
        for viewport in [(390.0, 844.0), (1280.0, 720.0), (1800.0, 1000.0)] {
            let scene = OpeningStory::sample(0.0, viewport, 800.0, 1400.0);
            assert_eq!(scene.hero_exit, 0.0);
        }
    }

    #[test]
    fn scroll_never_changes_extents_and_reverse_sampling_is_deterministic() {
        let base = OpeningStory::sample(0.0, (1280.0, 720.0), 800.0, 1400.0);
        for scroll in [-100.0, 400.0, 800.0, 1400.0, 10_000.0, 800.0, 0.0] {
            let scene = OpeningStory::sample(scroll, (1280.0, 720.0), 800.0, 1400.0);
            assert_eq!(scene.hero_span, base.hero_span);
            assert_eq!(scene.runtime_span, base.runtime_span);
            assert!((0.0..=100.0).contains(&scene.runtime_progress));
        }
        let arrival = OpeningStory::sample(600.0, (1280.0, 720.0), 800.0, 1400.0);
        assert!(arrival.hero_exit > 0.0 && arrival.hero_exit < 100.0);
        assert!(arrival.runtime_progress > 0.0 && arrival.runtime_progress < 50.0);
    }

    #[test]
    fn tall_viewports_do_not_add_empty_screenfuls() {
        let scene = OpeningStory::sample(0.0, (1280.0, 1600.0), 800.0, 1400.0);
        assert_eq!(scene.hero_span, 896.0);
        assert_eq!(scene.runtime_span, 1560.0);
    }

    #[test]
    fn later_chapters_have_independent_clamped_reversible_clocks() {
        let sample = |y| section_progress(y, 800.0, 2000.0, 1200.0);
        assert_eq!(sample(0.0), 0.0);
        assert_eq!(sample(4000.0), 100.0);
        let midway = sample(2100.0);
        assert!((35.0..65.0).contains(&midway));
        assert!(sample(1800.0) < midway);
        assert_eq!(sample(2100.0), midway);
        assert!(section_progress(0.0, 0.0, 0.0, 0.0).is_finite());
    }
}
