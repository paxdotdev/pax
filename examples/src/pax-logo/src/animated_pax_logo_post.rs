#![allow(unused_imports)]

use pax_kit::*;

const MID_X: f64 = 50.0768;
const TOP_BACK_LEFT: Point = Point::new(MID_X, 0.0);
const TOP_BACK_RIGHT: Point = Point::new(100.0, 5.2404);
const TOP_FRONT_LEFT: Point = Point::new(0.0, 5.622);
const TOP_FRONT_RIGHT: Point = Point::new(MID_X, 10.4807);
const BOTTOM_LEFT: Point = Point::new(0.0, 93.3227);
const BOTTOM_RIGHT: Point = Point::new(MID_X, 100.0);

// Keep authored overshoot inside the primitive bounds; presentation height restores final scale.
const CANVAS_Y_SCALE: f64 = 0.96;
const COMPONENT_WIDTH_PX: f64 = 137.23;
const COMPONENT_HEIGHT_PX: f64 = 463.95;
const X_PX_PER_PERCENT: f64 = COMPONENT_WIDTH_PX / 100.0;
const Y_PX_PER_PERCENT: f64 = COMPONENT_HEIGHT_PX * CANVAS_Y_SCALE / 100.0;

const ROLL_END_MS: f64 = 300.0;
const FALL_END_MS: f64 = 820.0;
const WHIP_PEAK_MS: f64 = 875.0;
const SETTLE_END_MS: f64 = 1080.0;
const INITIAL_ROLL_RADIUS_PX: f64 = 13.0;

#[pax]
#[custom(Default)]
#[file("animated_pax_logo_post.pax")]
pub struct AnimatedPaxLogoPost {
    pub fill: Property<Fill>,
    pub playhead: Property<f64>,
    pub top_fabric_elements: Property<Vec<PathElement>>,
    pub fabric_elements: Property<Vec<PathElement>>,
    pub fabric_opacity: Property<f64>,
    pub roll_start_cap_elements: Property<Vec<PathElement>>,
    pub roll_body_elements: Property<Vec<PathElement>>,
    pub roll_end_cap_elements: Property<Vec<PathElement>>,
}

impl Default for AnimatedPaxLogoPost {
    fn default() -> Self {
        Self {
            fill: Property::new(Fill::Solid(Color::BLACK)),
            playhead: Property::new(0.0),
            top_fabric_elements: Property::new(top_fabric_path_at(0.0)),
            fabric_elements: Property::new(fabric_path_at(0.0)),
            fabric_opacity: Property::new(0.0),
            roll_start_cap_elements: Property::new(roll_start_cap_path_at(0.0)),
            roll_body_elements: Property::new(roll_body_path_at(0.0)),
            roll_end_cap_elements: Property::new(roll_end_cap_path_at(0.0)),
        }
    }
}

impl AnimatedPaxLogoPost {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        let top_playhead = self.playhead.clone();
        let top_dependencies = [top_playhead.untyped()];
        let top_fabric_elements = Property::computed(
            move || top_fabric_path_at(top_playhead.get()),
            &top_dependencies,
        );
        self.top_fabric_elements.replace_with(top_fabric_elements);

        let fabric_playhead = self.playhead.clone();
        let fabric_dependencies = [fabric_playhead.untyped()];
        let fabric_elements = Property::computed(
            move || fabric_path_at(fabric_playhead.get()),
            &fabric_dependencies,
        );
        self.fabric_elements.replace_with(fabric_elements);

        let fabric_opacity_playhead = self.playhead.clone();
        let fabric_opacity_dependencies = [fabric_opacity_playhead.untyped()];
        let fabric_opacity = Property::computed(
            move || fabric_opacity_at(fabric_opacity_playhead.get()),
            &fabric_opacity_dependencies,
        );
        self.fabric_opacity.replace_with(fabric_opacity);

        let start_cap_playhead = self.playhead.clone();
        let start_cap_dependencies = [start_cap_playhead.untyped()];
        let roll_start_cap_elements = Property::computed(
            move || roll_start_cap_path_at(start_cap_playhead.get()),
            &start_cap_dependencies,
        );
        self.roll_start_cap_elements
            .replace_with(roll_start_cap_elements);

        let body_playhead = self.playhead.clone();
        let body_dependencies = [body_playhead.untyped()];
        let roll_body_elements = Property::computed(
            move || roll_body_path_at(body_playhead.get()),
            &body_dependencies,
        );
        self.roll_body_elements.replace_with(roll_body_elements);

        let end_cap_playhead = self.playhead.clone();
        let end_cap_dependencies = [end_cap_playhead.untyped()];
        let roll_end_cap_elements = Property::computed(
            move || roll_end_cap_path_at(end_cap_playhead.get()),
            &end_cap_dependencies,
        );
        self.roll_end_cap_elements
            .replace_with(roll_end_cap_elements);
    }
}

#[derive(Clone, Copy, Debug)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn lerp(self, other: Self, t: f64) -> Self {
        Self::new(lerp(self.x, other.x, t), lerp(self.y, other.y, t))
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    fn scale(self, amount: f64) -> Self {
        Self::new(self.x * amount, self.y * amount)
    }
}

#[derive(Clone, Copy)]
struct PostMotion {
    rolling: f64,
    extension: f64,
    wave: f64,
    swing: f64,
    whip: f64,
    fall: f64,
    roll_axis_start: Point,
    roll_axis_end: Point,
    roll_radius_px: f64,
}

fn motion_at(playhead_ms: f64) -> PostMotion {
    let rolling = smoothstep(progress(playhead_ms, 0.0, ROLL_END_MS));
    let fall = progress(playhead_ms, ROLL_END_MS, FALL_END_MS);
    let extension = if playhead_ms < ROLL_END_MS {
        0.0
    } else {
        ease_in_quad(fall)
    };
    let descent_envelope = (fall * std::f64::consts::PI).sin() * (1.0 - fall * 0.35);
    let wave = (fall * std::f64::consts::PI * 2.5).sin() * descent_envelope * 9.0;
    let swing = -(fall * std::f64::consts::PI).sin() * (1.0 - fall) * 14.0;
    let whip = if playhead_ms < FALL_END_MS {
        0.0
    } else if playhead_ms < WHIP_PEAK_MS {
        lerp(
            0.0,
            -4.5,
            ease_out_cubic(progress(playhead_ms, FALL_END_MS, WHIP_PEAK_MS)),
        )
    } else if playhead_ms < SETTLE_END_MS {
        lerp(
            -4.5,
            0.0,
            ease_out_cubic(progress(playhead_ms, WHIP_PEAK_MS, SETTLE_END_MS)),
        )
    } else {
        0.0
    };

    let bottom_left = Point::new(
        lerp(TOP_FRONT_LEFT.x, BOTTOM_LEFT.x, extension) + swing + wave * 0.12 + whip,
        lerp(TOP_FRONT_LEFT.y, BOTTOM_LEFT.y, extension),
    );
    let bottom_right = Point::new(
        lerp(TOP_FRONT_RIGHT.x, BOTTOM_RIGHT.x, extension)
            + swing * 0.75
            + wave * 0.55
            + whip * 0.8,
        lerp(TOP_FRONT_RIGHT.y, BOTTOM_RIGHT.y, extension),
    );

    let (roll_axis_start, roll_axis_end, roll_radius_px) = if playhead_ms < ROLL_END_MS {
        (
            TOP_BACK_LEFT.lerp(TOP_FRONT_LEFT, rolling),
            TOP_BACK_RIGHT.lerp(TOP_FRONT_RIGHT, rolling),
            lerp(INITIAL_ROLL_RADIUS_PX, 6.5, ease_out_quad(rolling)),
        )
    } else {
        (bottom_left, bottom_right, lerp(6.5, 0.7, smoothstep(fall)))
    };

    PostMotion {
        rolling,
        extension,
        wave,
        swing,
        whip,
        fall,
        roll_axis_start,
        roll_axis_end,
        roll_radius_px,
    }
}

fn top_fabric_path_at(playhead_ms: f64) -> Vec<PathElement> {
    let reveal = motion_at(playhead_ms).rolling.max(0.002);
    let moving_left = TOP_BACK_LEFT.lerp(TOP_FRONT_LEFT, reveal);
    let moving_right = TOP_BACK_RIGHT.lerp(TOP_FRONT_RIGHT, reveal);

    vec![
        point_element(TOP_BACK_LEFT),
        PathElement::Line,
        point_element(TOP_BACK_RIGHT),
        PathElement::Line,
        point_element(moving_right),
        PathElement::Line,
        point_element(moving_left),
        PathElement::Close,
    ]
}

fn fabric_path_at(playhead_ms: f64) -> Vec<PathElement> {
    let motion = motion_at(playhead_ms);
    let visible_extension = motion.extension.max(0.002);
    let bottom_left = Point::new(
        lerp(TOP_FRONT_LEFT.x, BOTTOM_LEFT.x, visible_extension)
            + motion.swing
            + motion.wave * 0.12
            + motion.whip,
        lerp(TOP_FRONT_LEFT.y, BOTTOM_LEFT.y, visible_extension),
    );
    let bottom_right = Point::new(
        lerp(TOP_FRONT_RIGHT.x, BOTTOM_RIGHT.x, visible_extension)
            + motion.swing * 0.75
            + motion.wave * 0.55
            + motion.whip * 0.8,
        lerp(TOP_FRONT_RIGHT.y, BOTTOM_RIGHT.y, visible_extension),
    );
    let left_extent = bottom_left.y - TOP_FRONT_LEFT.y;
    let right_extent = bottom_right.y - TOP_FRONT_RIGHT.y;
    let roll_sag = radius_y_percent(motion.roll_radius_px) * (1.0 - motion.fall) * 0.65;

    vec![
        point_element(TOP_FRONT_LEFT),
        PathElement::Cubic(
            percent(motion.swing * 0.05 - motion.wave * 0.08),
            percent_y(TOP_FRONT_LEFT.y + left_extent * 0.28),
            percent(motion.swing * 0.52 + motion.wave * 0.32 + motion.whip * 0.55),
            percent_y(TOP_FRONT_LEFT.y + left_extent * 0.72),
        ),
        point_element(bottom_left),
        PathElement::Cubic(
            percent(lerp(bottom_left.x, bottom_right.x, 1.0 / 3.0)),
            percent_y(lerp(bottom_left.y, bottom_right.y, 1.0 / 3.0) + roll_sag),
            percent(lerp(bottom_left.x, bottom_right.x, 2.0 / 3.0)),
            percent_y(lerp(bottom_left.y, bottom_right.y, 2.0 / 3.0) + roll_sag),
        ),
        point_element(bottom_right),
        PathElement::Cubic(
            percent(
                TOP_FRONT_RIGHT.x + motion.swing * 0.70 + motion.wave * 0.90 + motion.whip * 0.65,
            ),
            percent_y(TOP_FRONT_RIGHT.y + right_extent * 0.70),
            percent(TOP_FRONT_RIGHT.x - motion.wave * 0.22),
            percent_y(TOP_FRONT_RIGHT.y + right_extent * 0.30),
        ),
        point_element(TOP_FRONT_RIGHT),
        PathElement::Close,
    ]
}

fn fabric_opacity_at(playhead_ms: f64) -> f64 {
    smoothstep(progress(playhead_ms, ROLL_END_MS, ROLL_END_MS + 50.0))
}

fn roll_start_cap_path_at(playhead_ms: f64) -> Vec<PathElement> {
    let motion = motion_at(playhead_ms);
    circle_path(motion.roll_axis_start, motion.roll_radius_px)
}

fn roll_end_cap_path_at(playhead_ms: f64) -> Vec<PathElement> {
    let motion = motion_at(playhead_ms);
    circle_path(motion.roll_axis_end, motion.roll_radius_px)
}

fn roll_body_path_at(playhead_ms: f64) -> Vec<PathElement> {
    let motion = motion_at(playhead_ms);
    let start = motion.roll_axis_start;
    let end = motion.roll_axis_end;
    let normal = normal_offset(start, end, motion.roll_radius_px);

    vec![
        point_element(start.add(normal)),
        PathElement::Line,
        point_element(end.add(normal)),
        PathElement::Line,
        point_element(end.add(normal.scale(-1.0))),
        PathElement::Line,
        point_element(start.add(normal.scale(-1.0))),
        PathElement::Close,
    ]
}

fn circle_path(center: Point, radius_px: f64) -> Vec<PathElement> {
    let rx = radius_x_percent(radius_px);
    let ry = radius_y_percent(radius_px);
    let kappa = 0.552_284_749_830_793_6;
    vec![
        point_element(Point::new(center.x + rx, center.y)),
        PathElement::Cubic(
            percent(center.x + rx),
            percent_y(center.y + ry * kappa),
            percent(center.x + rx * kappa),
            percent_y(center.y + ry),
        ),
        point_element(Point::new(center.x, center.y + ry)),
        PathElement::Cubic(
            percent(center.x - rx * kappa),
            percent_y(center.y + ry),
            percent(center.x - rx),
            percent_y(center.y + ry * kappa),
        ),
        point_element(Point::new(center.x - rx, center.y)),
        PathElement::Cubic(
            percent(center.x - rx),
            percent_y(center.y - ry * kappa),
            percent(center.x - rx * kappa),
            percent_y(center.y - ry),
        ),
        point_element(Point::new(center.x, center.y - ry)),
        PathElement::Cubic(
            percent(center.x + rx * kappa),
            percent_y(center.y - ry),
            percent(center.x + rx),
            percent_y(center.y - ry * kappa),
        ),
        point_element(Point::new(center.x + rx, center.y)),
        PathElement::Close,
    ]
}

fn normal_offset(start: Point, end: Point, radius_px: f64) -> Point {
    let dx_px = (end.x - start.x) * X_PX_PER_PERCENT;
    let dy_px = (end.y - start.y) * Y_PX_PER_PERCENT;
    let length_px = (dx_px * dx_px + dy_px * dy_px).sqrt().max(f64::EPSILON);
    Point::new(
        (-dy_px / length_px * radius_px) / X_PX_PER_PERCENT,
        (dx_px / length_px * radius_px) / Y_PX_PER_PERCENT,
    )
}

#[cfg(test)]
fn screen_distance(start: Point, end: Point) -> f64 {
    let dx = (end.x - start.x) * X_PX_PER_PERCENT;
    let dy = (end.y - start.y) * Y_PX_PER_PERCENT;
    (dx * dx + dy * dy).sqrt()
}

fn radius_x_percent(radius_px: f64) -> f64 {
    radius_px / X_PX_PER_PERCENT
}

fn radius_y_percent(radius_px: f64) -> f64 {
    radius_px / Y_PX_PER_PERCENT
}

fn point_element(point: Point) -> PathElement {
    PathElement::Point(percent(point.x), percent_y(point.y))
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

fn ease_in_quad(t: f64) -> f64 {
    t * t
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
    fn post_paths_keep_stable_topologies_at_every_motion_phase() {
        for playhead in [0.0, 150.0, 300.0, 500.0, 700.0, 820.0, 875.0, 1080.0] {
            let top = top_fabric_path_at(playhead);
            let fabric = fabric_path_at(playhead);
            let start_cap = roll_start_cap_path_at(playhead);
            let body = roll_body_path_at(playhead);
            let end_cap = roll_end_cap_path_at(playhead);
            assert_eq!(top.len(), 8);
            assert_eq!(top.last(), Some(&PathElement::Close));
            assert_eq!(fabric.len(), 8);
            assert_eq!(fabric.last(), Some(&PathElement::Close));
            assert_eq!(start_cap.len(), 10);
            assert_eq!(start_cap.last(), Some(&PathElement::Close));
            assert_eq!(body.len(), 8);
            assert_eq!(body.last(), Some(&PathElement::Close));
            assert_eq!(end_cap.len(), 10);
            assert_eq!(end_cap.last(), Some(&PathElement::Close));
        }
    }

    #[test]
    fn top_face_is_revealed_before_the_descent() {
        assert_eq!(motion_at(0.0).extension, 0.0);
        assert_eq!(motion_at(ROLL_END_MS).rolling, 1.0);
        assert_eq!(motion_at(ROLL_END_MS).extension, 0.0);
        assert_eq!(fabric_opacity_at(ROLL_END_MS), 0.0);
        assert_eq!(fabric_opacity_at(ROLL_END_MS + 50.0), 1.0);
        assert!(motion_at(500.0).extension > 0.0);
    }

    #[test]
    fn capsule_keeps_its_axis_spanning_the_prism_edge() {
        let expected = screen_distance(TOP_BACK_LEFT, TOP_BACK_RIGHT);
        for playhead in [0.0, 75.0, 150.0, 225.0, 299.0] {
            let motion = motion_at(playhead);
            let length = screen_distance(motion.roll_axis_start, motion.roll_axis_end);
            assert!((length - expected).abs() < 0.5);
        }
    }

    #[test]
    fn settled_post_matches_the_original_rigid_extents() {
        let path = fabric_path_at(SETTLE_END_MS);
        assert_eq!(path[2], point_element(BOTTOM_LEFT));
        assert_eq!(path[4], point_element(BOTTOM_RIGHT));
    }

    #[test]
    fn spool_shrinks_and_the_fall_has_no_vertical_rebound() {
        assert_eq!(motion_at(0.0).roll_radius_px, INITIAL_ROLL_RADIUS_PX);
        assert!(motion_at(0.0).roll_radius_px > motion_at(FALL_END_MS).roll_radius_px);

        let mut previous_extension = motion_at(ROLL_END_MS).extension;
        for playhead in (320..=820).step_by(20) {
            let extension = motion_at(playhead as f64).extension;
            assert!(extension >= previous_extension);
            previous_extension = extension;
        }
        assert_eq!(motion_at(SETTLE_END_MS).extension, 1.0);
    }

    #[test]
    fn lower_edge_whips_once_and_returns_to_the_wall() {
        assert_eq!(motion_at(FALL_END_MS).whip, 0.0);
        assert!(motion_at(WHIP_PEAK_MS).whip < 0.0);
        assert_eq!(motion_at(SETTLE_END_MS).whip, 0.0);
    }
}
