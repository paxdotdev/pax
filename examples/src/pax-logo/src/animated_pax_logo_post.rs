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
const CANVAS_X_PADDING_PX: f64 = 16.0;
const CANVAS_WIDTH_PX: f64 = COMPONENT_WIDTH_PX + CANVAS_X_PADDING_PX * 2.0;
const X_PX_PER_PERCENT: f64 = COMPONENT_WIDTH_PX / 100.0;
const Y_PX_PER_PERCENT: f64 = COMPONENT_HEIGHT_PX * CANVAS_Y_SCALE / 100.0;

const INITIAL_ROLL_RADIUS_PX: f64 = 13.0;

#[pax]
#[custom(Default)]
#[file("animated_pax_logo_post.pax")]
pub struct AnimatedPaxLogoPost {
    pub fill: Property<Fill>,
    pub rolling: Property<f64>,
    pub extension: Property<f64>,
    pub wave: Property<f64>,
    pub swing: Property<f64>,
    pub whip: Property<f64>,
    pub fall: Property<f64>,
    pub roll_radius_px: Property<f64>,
    pub top_fabric_elements: Property<Vec<PathElement>>,
    pub fabric_elements: Property<Vec<PathElement>>,
    pub fabric_opacity: Property<f64>,
    pub roll_start_cap_elements: Property<Vec<PathElement>>,
    pub roll_body_elements: Property<Vec<PathElement>>,
    pub roll_end_cap_elements: Property<Vec<PathElement>>,
}

impl Default for AnimatedPaxLogoPost {
    fn default() -> Self {
        let motion = motion_from_controls(MotionControls::default());
        Self {
            fill: Property::new(Fill::Solid(Color::BLACK)),
            rolling: Property::new(0.0),
            extension: Property::new(0.0),
            wave: Property::new(0.0),
            swing: Property::new(0.0),
            whip: Property::new(0.0),
            fall: Property::new(0.0),
            roll_radius_px: Property::new(INITIAL_ROLL_RADIUS_PX),
            top_fabric_elements: Property::new(top_fabric_path(motion)),
            fabric_elements: Property::new(fabric_path(motion)),
            fabric_opacity: Property::new(0.0),
            roll_start_cap_elements: Property::new(roll_start_cap_path(motion)),
            roll_body_elements: Property::new(roll_body_path(motion)),
            roll_end_cap_elements: Property::new(roll_end_cap_path(motion)),
        }
    }
}

impl AnimatedPaxLogoPost {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        let rolling = self.rolling.clone();
        let extension = self.extension.clone();
        let wave = self.wave.clone();
        let swing = self.swing.clone();
        let whip = self.whip.clone();
        let fall = self.fall.clone();
        let roll_radius_px = self.roll_radius_px.clone();
        let motion_dependencies = [
            rolling.untyped(),
            extension.untyped(),
            wave.untyped(),
            swing.untyped(),
            whip.untyped(),
            fall.untyped(),
            roll_radius_px.untyped(),
        ];
        let motion = Property::computed(
            move || {
                motion_from_controls(MotionControls {
                    rolling: rolling.get(),
                    extension: extension.get(),
                    wave: wave.get(),
                    swing: swing.get(),
                    whip: whip.get(),
                    fall: fall.get(),
                    roll_radius_px: roll_radius_px.get(),
                })
            },
            &motion_dependencies,
        );

        self.top_fabric_elements
            .replace_with(computed_path(&motion, top_fabric_path));
        self.fabric_elements
            .replace_with(computed_path(&motion, fabric_path));
        self.roll_start_cap_elements
            .replace_with(computed_path(&motion, roll_start_cap_path));
        self.roll_body_elements
            .replace_with(computed_path(&motion, roll_body_path));
        self.roll_end_cap_elements
            .replace_with(computed_path(&motion, roll_end_cap_path));
    }
}

fn computed_path(
    motion: &Property<PostMotion>,
    build: fn(PostMotion) -> Vec<PathElement>,
) -> Property<Vec<PathElement>> {
    let motion = motion.clone();
    let dependencies = [motion.untyped()];
    Property::computed(move || build(motion.get()), &dependencies)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
struct MotionControls {
    rolling: f64,
    extension: f64,
    wave: f64,
    swing: f64,
    whip: f64,
    fall: f64,
    roll_radius_px: f64,
}

impl Default for MotionControls {
    fn default() -> Self {
        Self {
            rolling: 0.0,
            extension: 0.0,
            wave: 0.0,
            swing: 0.0,
            whip: 0.0,
            fall: 0.0,
            roll_radius_px: INITIAL_ROLL_RADIUS_PX,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct PostMotion {
    rolling: f64,
    extension: f64,
    wave: f64,
    swing: f64,
    whip: f64,
    fall: f64,
    fabric_bottom_left: Point,
    fabric_bottom_right: Point,
    roll_axis_start: Point,
    roll_axis_end: Point,
    roll_radius_px: f64,
}

impl Interpolatable for PostMotion {}

fn motion_from_controls(controls: MotionControls) -> PostMotion {
    let base_bottom_left = Point::new(
        lerp(TOP_FRONT_LEFT.x, BOTTOM_LEFT.x, controls.extension)
            + controls.swing
            + controls.wave * 0.12
            + controls.whip,
        lerp(TOP_FRONT_LEFT.y, BOTTOM_LEFT.y, controls.extension),
    );
    let base_bottom_right = Point::new(
        lerp(TOP_FRONT_RIGHT.x, BOTTOM_RIGHT.x, controls.extension)
            + controls.swing * 0.75
            + controls.wave * 0.55
            + controls.whip * 0.8,
        lerp(TOP_FRONT_RIGHT.y, BOTTOM_RIGHT.y, controls.extension),
    );
    let (roll_axis_start, roll_axis_end) = if controls.fall <= f64::EPSILON {
        (
            TOP_BACK_LEFT.lerp(TOP_FRONT_LEFT, controls.rolling),
            TOP_BACK_RIGHT.lerp(TOP_FRONT_RIGHT, controls.rolling),
        )
    } else {
        (base_bottom_left, base_bottom_right)
    };

    PostMotion {
        rolling: controls.rolling,
        extension: controls.extension,
        wave: controls.wave,
        swing: controls.swing,
        whip: controls.whip,
        fall: controls.fall,
        fabric_bottom_left: base_bottom_left,
        fabric_bottom_right: base_bottom_right,
        roll_axis_start,
        roll_axis_end,
        roll_radius_px: controls.roll_radius_px,
    }
}

fn top_fabric_path(motion: PostMotion) -> Vec<PathElement> {
    let reveal = motion.rolling.max(0.002);
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

fn fabric_path(motion: PostMotion) -> Vec<PathElement> {
    let bottom_left = motion.fabric_bottom_left;
    let bottom_right = motion.fabric_bottom_right;
    let left_extent = bottom_left.y - TOP_FRONT_LEFT.y;
    let right_extent = bottom_right.y - TOP_FRONT_RIGHT.y;
    let roll_sag = radius_y_percent(motion.roll_radius_px) * (1.0 - motion.fall) * 0.65;
    let left_upper = Point::new(
        motion.swing * 0.05 - motion.wave * 0.08,
        TOP_FRONT_LEFT.y + left_extent / 3.0,
    );
    let left_middle = Point::new(
        motion.swing * 0.52 + motion.wave * 0.32 + motion.whip * 0.55,
        TOP_FRONT_LEFT.y + left_extent * 2.0 / 3.0,
    );
    let right_upper = Point::new(
        TOP_FRONT_RIGHT.x - motion.wave * 0.22,
        TOP_FRONT_RIGHT.y + right_extent / 3.0,
    );
    let right_middle = Point::new(
        TOP_FRONT_RIGHT.x + motion.swing * 0.70 + motion.wave * 0.90 + motion.whip * 0.65,
        TOP_FRONT_RIGHT.y + right_extent * 2.0 / 3.0,
    );
    let left_edge = [TOP_FRONT_LEFT, left_upper, left_middle, bottom_left];
    let right_edge = [bottom_right, right_middle, right_upper, TOP_FRONT_RIGHT];

    let mut path = vec![point_element(TOP_FRONT_LEFT)];
    append_catmull_rom_edge(&mut path, &left_edge);
    path.extend([
        PathElement::Cubic(
            percent_x(lerp(bottom_left.x, bottom_right.x, 1.0 / 3.0)),
            percent_y(lerp(bottom_left.y, bottom_right.y, 1.0 / 3.0) + roll_sag),
            percent_x(lerp(bottom_left.x, bottom_right.x, 2.0 / 3.0)),
            percent_y(lerp(bottom_left.y, bottom_right.y, 2.0 / 3.0) + roll_sag),
        ),
        point_element(bottom_right),
    ]);
    append_catmull_rom_edge(&mut path, &right_edge);
    path.push(PathElement::Close);
    path
}

fn append_catmull_rom_edge(path: &mut Vec<PathElement>, points: &[Point; 4]) {
    for index in 0..3 {
        let previous = if index == 0 {
            points[index]
        } else {
            points[index - 1]
        };
        let start = points[index];
        let end = points[index + 1];
        let next = if index == 2 {
            points[index + 1]
        } else {
            points[index + 2]
        };
        let control_1 = Point::new(
            start.x + (end.x - previous.x) / 6.0,
            start.y + (end.y - previous.y) / 6.0,
        );
        let control_2 = Point::new(
            end.x - (next.x - start.x) / 6.0,
            end.y - (next.y - start.y) / 6.0,
        );
        path.push(PathElement::Cubic(
            percent_x(control_1.x),
            percent_y(control_1.y),
            percent_x(control_2.x),
            percent_y(control_2.y),
        ));
        path.push(point_element(end));
    }
}

fn roll_start_cap_path(motion: PostMotion) -> Vec<PathElement> {
    circle_path(motion.roll_axis_start, motion.roll_radius_px)
}

fn roll_end_cap_path(motion: PostMotion) -> Vec<PathElement> {
    circle_path(motion.roll_axis_end, motion.roll_radius_px)
}

fn roll_body_path(motion: PostMotion) -> Vec<PathElement> {
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
            percent_x(center.x + rx),
            percent_y(center.y + ry * kappa),
            percent_x(center.x + rx * kappa),
            percent_y(center.y + ry),
        ),
        point_element(Point::new(center.x, center.y + ry)),
        PathElement::Cubic(
            percent_x(center.x - rx * kappa),
            percent_y(center.y + ry),
            percent_x(center.x - rx),
            percent_y(center.y + ry * kappa),
        ),
        point_element(Point::new(center.x - rx, center.y)),
        PathElement::Cubic(
            percent_x(center.x - rx),
            percent_y(center.y - ry * kappa),
            percent_x(center.x - rx * kappa),
            percent_y(center.y - ry),
        ),
        point_element(Point::new(center.x, center.y - ry)),
        PathElement::Cubic(
            percent_x(center.x + rx * kappa),
            percent_y(center.y - ry),
            percent_x(center.x + rx),
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
    PathElement::Point(percent_x(point.x), percent_y(point.y))
}

fn percent_x(value: f64) -> Size {
    percent((value * X_PX_PER_PERCENT + CANVAS_X_PADDING_PX) / CANVAS_WIDTH_PX * 100.0)
}

fn percent(value: f64) -> Size {
    Size::Percent(Numeric::F64(value))
}

fn percent_y(value: f64) -> Size {
    percent(value * CANVAS_Y_SCALE)
}

fn lerp(start: f64, end: f64, t: f64) -> f64 {
    start + (end - start) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top_controls(rolling: f64, radius: f64) -> MotionControls {
        MotionControls {
            rolling,
            roll_radius_px: radius,
            ..Default::default()
        }
    }

    fn fall_controls(
        extension: f64,
        fall: f64,
        wave: f64,
        swing: f64,
        whip: f64,
        radius: f64,
    ) -> MotionControls {
        MotionControls {
            rolling: 1.0,
            extension,
            fall,
            wave,
            swing,
            whip,
            roll_radius_px: radius,
        }
    }

    #[test]
    fn post_paths_keep_stable_topologies_at_every_motion_phase() {
        let samples = [
            top_controls(0.0, INITIAL_ROLL_RADIUS_PX),
            top_controls(0.5, 8.125),
            top_controls(1.0, 6.5),
            fall_controls(0.25, 0.5, -5.2503, -7.0, 0.0, 3.6),
            fall_controls(1.0, 1.0, 0.0, 0.0, 0.0, 0.7),
            fall_controls(1.0, 1.0, 0.0, 0.0, -4.5, 0.7),
        ];

        for controls in samples {
            let motion = motion_from_controls(controls);
            let top = top_fabric_path(motion);
            let fabric = fabric_path(motion);
            let start_cap = roll_start_cap_path(motion);
            let body = roll_body_path(motion);
            let end_cap = roll_end_cap_path(motion);
            assert_eq!(top.len(), 8);
            assert_eq!(top.last(), Some(&PathElement::Close));
            assert_eq!(fabric.len(), 16);
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
        let start = motion_from_controls(top_controls(0.0, INITIAL_ROLL_RADIUS_PX));
        let rolled = motion_from_controls(top_controls(1.0, 6.5));
        let falling = motion_from_controls(fall_controls(0.25, 0.5, 0.0, 0.0, 0.0, 3.6));

        assert_eq!(start.extension, 0.0);
        assert_eq!(rolled.rolling, 1.0);
        assert_eq!(rolled.extension, 0.0);
        assert!(falling.extension > 0.0);
    }

    #[test]
    fn capsule_keeps_its_axis_spanning_the_prism_edge() {
        let expected = screen_distance(TOP_BACK_LEFT, TOP_BACK_RIGHT);
        for rolling in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let motion = motion_from_controls(top_controls(rolling, 10.0));
            let length = screen_distance(motion.roll_axis_start, motion.roll_axis_end);
            assert!((length - expected).abs() < 0.5);
        }
    }

    #[test]
    fn settled_post_matches_the_original_rigid_extents() {
        let motion = motion_from_controls(fall_controls(1.0, 1.0, 0.0, 0.0, 0.0, 0.7));
        let path = fabric_path(motion);
        assert_eq!(path[6], point_element(BOTTOM_LEFT));
        assert_eq!(path[8], point_element(BOTTOM_RIGHT));
    }

    #[test]
    fn scalar_controls_drive_spool_shrink_and_full_extension() {
        let start = motion_from_controls(top_controls(0.0, INITIAL_ROLL_RADIUS_PX));
        let settled = motion_from_controls(fall_controls(1.0, 1.0, 0.0, 0.0, 0.0, 0.7));

        assert_eq!(start.roll_radius_px, INITIAL_ROLL_RADIUS_PX);
        assert!(start.roll_radius_px > settled.roll_radius_px);
        assert_eq!(settled.extension, 1.0);
    }

    #[test]
    fn lower_edge_whips_once_and_returns_to_the_wall() {
        let before = motion_from_controls(fall_controls(1.0, 1.0, 0.0, 0.0, 0.0, 0.7));
        let peak = motion_from_controls(fall_controls(1.0, 1.0, 0.0, 0.0, -4.5, 0.7));
        let settled = motion_from_controls(fall_controls(1.0, 1.0, 0.0, 0.0, 0.0, 0.7));

        assert_eq!(before.whip, 0.0);
        assert!(peak.whip < 0.0);
        assert_eq!(settled.whip, 0.0);
    }

    #[test]
    fn front_edge_spool_cap_has_horizontal_drawing_headroom() {
        let radius_percent = radius_x_percent(INITIAL_ROLL_RADIUS_PX);
        let leftmost_source_x = TOP_FRONT_LEFT.x - radius_percent;
        let leftmost_canvas_x =
            (leftmost_source_x * X_PX_PER_PERCENT + CANVAS_X_PADDING_PX) / CANVAS_WIDTH_PX;
        assert!(leftmost_canvas_x > 0.0);
    }
}
