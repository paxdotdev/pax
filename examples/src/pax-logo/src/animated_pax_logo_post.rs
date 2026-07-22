#![allow(unused_imports)]

use pax_kit::*;

const TOP_LEFT_Y: f64 = 5.622;
const TOP_RIGHT_Y: f64 = 10.4807;
const BOTTOM_LEFT_Y: f64 = 93.3227;
const BOTTOM_RIGHT_Y: f64 = 100.0;
const MID_X: f64 = 50.0768;
// Keep the overshoot inside the primitive's bounds; presentation height restores final scale.
const CANVAS_Y_SCALE: f64 = 0.96;

#[pax]
#[custom(Default)]
#[file("animated_pax_logo_post.pax")]
pub struct AnimatedPaxLogoPost {
    pub fill: Property<Fill>,
    pub playhead: Property<f64>,
    pub path_elements: Property<Vec<PathElement>>,
}

impl Default for AnimatedPaxLogoPost {
    fn default() -> Self {
        Self {
            fill: Property::new(Fill::Solid(Color::BLACK)),
            playhead: Property::new(0.0),
            path_elements: Property::new(post_path_at(0.0)),
        }
    }
}

impl AnimatedPaxLogoPost {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        let playhead = self.playhead.clone();
        let dependencies = [playhead.untyped()];
        let path_elements = Property::computed(move || post_path_at(playhead.get()), &dependencies);
        self.path_elements.replace_with(path_elements);
    }
}

fn post_path_at(playhead_ms: f64) -> Vec<PathElement> {
    let extension = if playhead_ms < 110.0 {
        lerp(0.075, 0.045, smoothstep(progress(playhead_ms, 0.0, 110.0)))
    } else if playhead_ms < 650.0 {
        lerp(
            0.045,
            1.04,
            ease_out_cubic(progress(playhead_ms, 110.0, 650.0)),
        )
    } else if playhead_ms < 760.0 {
        lerp(
            1.04,
            0.97,
            ease_out_quad(progress(playhead_ms, 650.0, 760.0)),
        )
    } else if playhead_ms < 900.0 {
        lerp(
            0.97,
            1.012,
            ease_out_quad(progress(playhead_ms, 760.0, 900.0)),
        )
    } else if playhead_ms < 1020.0 {
        lerp(1.012, 1.0, smoothstep(progress(playhead_ms, 900.0, 1020.0)))
    } else {
        1.0
    };

    let unfurl = progress(playhead_ms, 110.0, 650.0);
    let wave_envelope = (1.0 - unfurl) * (unfurl * std::f64::consts::PI).sin();
    let wave = (unfurl * std::f64::consts::PI * 4.0).sin() * wave_envelope * 20.0;
    let billow = wave_envelope * 4.0;

    let bottom_left_y = lerp(TOP_LEFT_Y, BOTTOM_LEFT_Y, extension);
    let bottom_right_y = lerp(TOP_RIGHT_Y, BOTTOM_RIGHT_Y, extension);
    let left_extent = bottom_left_y - TOP_LEFT_Y;
    let right_extent = bottom_right_y - TOP_RIGHT_Y;
    let bottom_left_x = wave * 0.18;
    let bottom_right_x = MID_X + wave * 0.72;

    vec![
        PathElement::Point(percent(0.0), percent_y(TOP_LEFT_Y)),
        PathElement::Cubic(
            percent(-wave * 0.12),
            percent_y(TOP_LEFT_Y + left_extent * 0.28),
            percent(wave * 0.42),
            percent_y(TOP_LEFT_Y + left_extent * 0.72),
        ),
        PathElement::Point(percent(bottom_left_x), percent_y(bottom_left_y)),
        PathElement::Cubic(
            percent(16.69 + wave * 0.30),
            percent_y(lerp(bottom_left_y, bottom_right_y, 1.0 / 3.0) + billow),
            percent(33.38 + wave * 0.58),
            percent_y(lerp(bottom_left_y, bottom_right_y, 2.0 / 3.0) + billow * 0.7),
        ),
        PathElement::Point(percent(bottom_right_x), percent_y(bottom_right_y)),
        PathElement::Cubic(
            percent(MID_X + wave * 1.15),
            percent_y(TOP_RIGHT_Y + right_extent * 0.70),
            percent(MID_X - wave * 0.35),
            percent_y(TOP_RIGHT_Y + right_extent * 0.30),
        ),
        PathElement::Point(percent(MID_X), percent_y(TOP_RIGHT_Y)),
        PathElement::Line,
        PathElement::Point(percent(100.0), percent_y(5.2404)),
        PathElement::Line,
        PathElement::Point(percent(MID_X), percent_y(0.0)),
        PathElement::Close,
    ]
}

fn percent(value: f64) -> Size {
    Size::Percent(Numeric::F64(value))
}

fn percent_y(value: f64) -> Size {
    percent(value * CANVAS_Y_SCALE)
}

fn progress(value: f64, start: f64, end: f64) -> f64 {
    ((value - start) / (end - start)).clamp(0.0, 1.0)
}

fn lerp(start: f64, end: f64, t: f64) -> f64 {
    start + (end - start) * t
}

fn ease_out_quad(t: f64) -> f64 {
    1.0 - (1.0 - t) * (1.0 - t)
}

fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}

fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_path_keeps_a_stable_topology_at_every_motion_phase() {
        for playhead in [0.0, 110.0, 300.0, 500.0, 650.0, 760.0, 900.0, 1020.0] {
            let path = post_path_at(playhead);
            assert_eq!(path.len(), 12);
            assert_eq!(path.last(), Some(&PathElement::Close));
        }
    }

    #[test]
    fn settled_post_matches_the_original_rigid_extents() {
        let path = post_path_at(1020.0);
        assert_eq!(
            path[2],
            PathElement::Point(percent(0.0), percent_y(BOTTOM_LEFT_Y))
        );
        assert_eq!(
            path[4],
            PathElement::Point(percent(MID_X), percent_y(BOTTOM_RIGHT_Y))
        );
    }
}
