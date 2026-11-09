// One reversible clock for two stationary owners. The small overlap happens at
// viewport edges, never by transferring a shader capture between render layers.
pub(crate) const DURATION_MS: f64 = 800.0;

fn segment(progress: f64, start: f64, end: f64) -> f64 {
    let t = ((progress - start) / (end - start)).clamp(0.0, 1.0);
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) * 0.5
    }
}

pub(crate) fn hero_departure(progress: f64) -> f64 {
    segment(progress, 0.0, 0.50)
}

pub(crate) fn nav_reveal(progress: f64) -> f64 {
    segment(progress, 0.30, 0.55)
}

pub(crate) fn nav_entry(progress: f64) -> f64 {
    segment(progress, 0.44, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_settled_and_clamped() {
        for sample in [hero_departure, nav_reveal, nav_entry] {
            assert_eq!(sample(-1.0), 0.0);
            assert_eq!(sample(0.0), 0.0);
            assert_eq!(sample(1.0), 1.0);
            assert_eq!(sample(2.0), 1.0);
            for step in 1..=100 {
                assert!(sample(step as f64 / 100.0) >= sample((step - 1) as f64 / 100.0));
            }
        }
    }

    #[test]
    fn edge_crossings_overlap_but_nav_retreat_waits_for_its_logo() {
        assert!(hero_departure(0.47) < 1.0);
        assert!(nav_entry(0.47) > 0.0);
        assert_eq!(nav_reveal(0.55), 1.0);
        assert!(nav_entry(0.55) < 0.08);
        assert_eq!(nav_entry(0.30), 0.0);
    }

    #[test]
    fn website_keeps_both_logos_in_their_own_contexts() {
        let hero = include_str!("hero_section.rs");
        assert!(!hero.contains("move_to_"));
        assert!(!hero.contains(".animated("));
        assert!(include_str!("site_shell.pax").contains("id=nav_portal_logo"));
    }
}
