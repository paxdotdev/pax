#![allow(unused_imports)]

use pax_kit::*;

const CANVAS_WIDTH_PX: f64 = 920.72;
const CANVAS_HEIGHT_PX: f64 = 463.95;

const BANNER_LEFT_X: f64 = 137.23;
const BANNER_TOP_Y: f64 = 23.34;
const BANNER_SHOULDER_X: f64 = 789.81;
const BANNER_TIP_X: f64 = 892.72;
const BANNER_TIP_Y: f64 = 168.57;
const BANNER_BOTTOM_Y: f64 = 313.20;

const A_CENTER: Point = Point::new(413.60, 168.77);
const A_BRAKE_PIVOT: Point = Point::new(530.43, 283.94);
const P_COUNTER_CENTER: Point = Point::new(189.01, 169.47);
const P_RING_CENTER: Point = Point::new(189.01, 168.305);
const P_RING_RADIUS_X_PX: f64 = 109.5;
const P_RING_RADIUS_Y_PX: f64 = 112.925;
const A_COUNTER_CENTER: Point = Point::new(412.46, 168.76);
const X_CENTER: Point = Point::new(648.34, 168.755);
const X_FINAL_Y_OFFSET_PX: f64 = 0.73;
const X_BRAKE_PIVOT: Point = Point::new(766.09, 282.99 + X_FINAL_Y_OFFSET_PX);
const P_COUNTER_RADIUS_PX: f64 = 53.51;
const P_INITIAL_SCALE: f64 = 0.28;
const A_COUNTER_RADIUS_PX: f64 = 54.61;
const A_INITIAL_OUTER_RADIUS_PX: f64 = 112.925;
const P_DISPATCH_TRANSLATION_X: f64 = -91.666_666_666_666_67;
#[cfg(test)]
const P_DISPATCH_SCALE: f64 = 0.4;
const A_INITIAL_SCALE: f64 = 0.16;
const A_INITIAL_TRANSLATION_X: f64 = P_COUNTER_CENTER.x + P_DISPATCH_TRANSLATION_X - A_CENTER.x;
const A_SIDEBAR_LEFT_X: f64 = 465.49;
const A_SIDEBAR_RIGHT_X: f64 = 530.43;
const X_INITIAL_SCALE: f64 = 0.28;
const X_INITIAL_TRANSLATION_X: f64 = (A_SIDEBAR_LEFT_X + A_SIDEBAR_RIGHT_X) * 0.5 - X_CENTER.x;
const X_INITIAL_TRANSLATION_Y: f64 = X_FINAL_Y_OFFSET_PX;
const X_INITIAL_ROTATION_DEG: f64 = -90.0;

#[pax]
#[custom(Default)]
#[file("animated_pax_logo_banner.pax")]
pub struct AnimatedPaxLogoBanner {
    pub fill: Property<Fill>,
    pub letter_fill: Property<Fill>,
    pub banner_progress: Property<f64>,
    pub banner_rustle_px: Property<f64>,
    pub banner_opacity: Property<f64>,
    pub p_counter_opacity: Property<f64>,
    pub p_counter_final_opacity: Property<f64>,
    pub p_counter_reveal_progress: Property<f64>,
    pub p_counter_x_px: Property<f64>,
    pub p_rotation_deg: Property<f64>,
    pub p_scale: Property<f64>,
    pub a_x_px: Property<f64>,
    pub a_rotation_deg: Property<f64>,
    pub a_scale: Property<f64>,
    pub a_brake_rotation_deg: Property<f64>,
    pub a_morph_progress: Property<f64>,
    pub a_opacity: Property<f64>,
    pub x_offset_px: Property<f64>,
    pub x_offset_y_px: Property<f64>,
    pub x_rotation_deg: Property<f64>,
    pub x_scale: Property<f64>,
    pub x_brake_rotation_deg: Property<f64>,
    pub x_opacity: Property<f64>,
    pub banner_elements: Property<Vec<PathElement>>,
    pub p_ring_elements: Property<Vec<PathElement>>,
    pub p_counter_elements: Property<Vec<PathElement>>,
    pub p_counter_final_elements: Property<Vec<PathElement>>,
    pub p_reveal_mask_elements: Property<Vec<PathElement>>,
    pub p_counter_reveal_mask_elements: Property<Vec<PathElement>>,
    pub a_elements: Property<Vec<PathElement>>,
    pub a_counter_elements: Property<Vec<PathElement>>,
    pub a_counter_reveal_mask_elements: Property<Vec<PathElement>>,
    pub x_elements: Property<Vec<PathElement>>,
}

impl Default for AnimatedPaxLogoBanner {
    fn default() -> Self {
        Self {
            fill: Property::new(Fill::Solid(Color::BLACK)),
            letter_fill: Property::new(Fill::Solid(Color::WHITE)),
            banner_progress: Property::new(0.0),
            banner_rustle_px: Property::new(0.0),
            banner_opacity: Property::new(0.0),
            p_counter_opacity: Property::new(1.0),
            p_counter_final_opacity: Property::new(0.0),
            p_counter_reveal_progress: Property::new(0.0),
            p_counter_x_px: Property::new(-110.0),
            p_rotation_deg: Property::new(-90.0),
            p_scale: Property::new(P_INITIAL_SCALE),
            a_x_px: Property::new(A_INITIAL_TRANSLATION_X),
            a_rotation_deg: Property::new(-90.0),
            a_scale: Property::new(A_INITIAL_SCALE),
            a_brake_rotation_deg: Property::new(0.0),
            a_morph_progress: Property::new(0.0),
            a_opacity: Property::new(0.0),
            x_offset_px: Property::new(X_INITIAL_TRANSLATION_X),
            x_offset_y_px: Property::new(X_INITIAL_TRANSLATION_Y),
            x_rotation_deg: Property::new(X_INITIAL_ROTATION_DEG),
            x_scale: Property::new(X_INITIAL_SCALE),
            x_brake_rotation_deg: Property::new(0.0),
            x_opacity: Property::new(0.0),
            banner_elements: Property::new(banner_path(0.0, 0.0)),
            p_ring_elements: Property::new(p_ring_path(Transform {
                translation_x: -110.0,
                rotation_deg: -90.0,
                scale: P_INITIAL_SCALE,
                brake_rotation_deg: 0.0,
            })),
            p_counter_elements: Property::new(p_counter_path(Transform {
                translation_x: -110.0,
                rotation_deg: -90.0,
                scale: P_INITIAL_SCALE,
                brake_rotation_deg: 0.0,
            })),
            p_counter_final_elements: Property::new(p_counter_path(Transform::default())),
            p_reveal_mask_elements: Property::new(p_reveal_mask_path()),
            p_counter_reveal_mask_elements: Property::new(p_counter_reveal_mask_path(0.0)),
            a_elements: Property::new(a_path(
                Transform {
                    translation_x: A_INITIAL_TRANSLATION_X,
                    rotation_deg: -90.0,
                    scale: A_INITIAL_SCALE,
                    brake_rotation_deg: 0.0,
                },
                0.0,
            )),
            a_counter_elements: Property::new(a_counter_path(
                Transform {
                    translation_x: A_INITIAL_TRANSLATION_X,
                    rotation_deg: -90.0,
                    scale: A_INITIAL_SCALE,
                    brake_rotation_deg: 0.0,
                },
                0.0,
            )),
            a_counter_reveal_mask_elements: Property::new(a_counter_reveal_mask_path()),
            x_elements: Property::new(x_path(XTransform::initial())),
        }
    }
}

impl AnimatedPaxLogoBanner {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        let banner_progress = self.banner_progress.clone();
        let banner_rustle_px = self.banner_rustle_px.clone();
        let banner_dependencies = [banner_progress.untyped(), banner_rustle_px.untyped()];
        self.banner_elements.replace_with(Property::computed(
            move || banner_path(banner_progress.get(), banner_rustle_px.get()),
            &banner_dependencies,
        ));

        let p_counter_x_px = self.p_counter_x_px.clone();
        let p_rotation_deg = self.p_rotation_deg.clone();
        let p_scale = self.p_scale.clone();
        let p_dependencies = [
            p_counter_x_px.untyped(),
            p_rotation_deg.untyped(),
            p_scale.untyped(),
        ];
        let p_transform = Property::computed(
            move || Transform {
                translation_x: p_counter_x_px.get(),
                rotation_deg: p_rotation_deg.get(),
                scale: p_scale.get(),
                brake_rotation_deg: 0.0,
            },
            &p_dependencies,
        );

        let p_transform_for_ring = p_transform.clone();
        self.p_ring_elements.replace_with(Property::computed(
            move || p_ring_path(p_transform_for_ring.get()),
            &[p_transform.untyped()],
        ));

        let p_transform_for_counter = p_transform.clone();
        self.p_counter_elements.replace_with(Property::computed(
            move || p_counter_path(p_transform_for_counter.get()),
            &[p_transform.untyped()],
        ));

        let p_counter_reveal_progress = self.p_counter_reveal_progress.clone();
        let p_counter_reveal_dependency = p_counter_reveal_progress.clone();
        self.p_counter_reveal_mask_elements
            .replace_with(Property::computed(
                move || p_counter_reveal_mask_path(p_counter_reveal_progress.get()),
                &[p_counter_reveal_dependency.untyped()],
            ));

        let a_x_px = self.a_x_px.clone();
        let a_rotation_deg = self.a_rotation_deg.clone();
        let a_scale = self.a_scale.clone();
        let a_brake_rotation_deg = self.a_brake_rotation_deg.clone();
        let a_morph_progress = self.a_morph_progress.clone();
        let transform_dependencies = [
            a_x_px.untyped(),
            a_rotation_deg.untyped(),
            a_scale.untyped(),
            a_brake_rotation_deg.untyped(),
        ];
        let a_transform = Property::computed(
            move || Transform {
                translation_x: a_x_px.get(),
                rotation_deg: a_rotation_deg.get(),
                scale: a_scale.get(),
                brake_rotation_deg: a_brake_rotation_deg.get(),
            },
            &transform_dependencies,
        );

        let a_transform_for_outer = a_transform.clone();
        let a_morph_for_outer = a_morph_progress.clone();
        self.a_elements.replace_with(Property::computed(
            move || a_path(a_transform_for_outer.get(), a_morph_for_outer.get()),
            &[a_transform.untyped(), a_morph_progress.untyped()],
        ));

        let a_transform_for_counter = a_transform.clone();
        let a_morph_for_counter = a_morph_progress.clone();
        self.a_counter_elements.replace_with(Property::computed(
            move || a_counter_path(a_transform_for_counter.get(), a_morph_for_counter.get()),
            &[a_transform.untyped(), a_morph_progress.untyped()],
        ));

        let x_offset_px = self.x_offset_px.clone();
        let x_offset_y_px = self.x_offset_y_px.clone();
        let x_rotation_deg = self.x_rotation_deg.clone();
        let x_scale = self.x_scale.clone();
        let x_brake_rotation_deg = self.x_brake_rotation_deg.clone();
        let x_dependencies = [
            x_offset_px.untyped(),
            x_offset_y_px.untyped(),
            x_rotation_deg.untyped(),
            x_scale.untyped(),
            x_brake_rotation_deg.untyped(),
        ];
        self.x_elements.replace_with(Property::computed(
            move || {
                x_path(XTransform {
                    translation_x: x_offset_px.get(),
                    translation_y: x_offset_y_px.get(),
                    rotation_deg: x_rotation_deg.get(),
                    scale: x_scale.get(),
                    brake_rotation_deg: x_brake_rotation_deg.get(),
                })
            },
            &x_dependencies,
        ));
    }
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
}

#[derive(Clone, Copy, Debug)]
struct Transform {
    translation_x: f64,
    rotation_deg: f64,
    scale: f64,
    brake_rotation_deg: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation_x: 0.0,
            rotation_deg: 0.0,
            scale: 1.0,
            brake_rotation_deg: 0.0,
        }
    }
}

impl Interpolatable for Transform {}

#[derive(Clone, Copy, Debug, PartialEq)]
struct XTransform {
    translation_x: f64,
    translation_y: f64,
    rotation_deg: f64,
    scale: f64,
    brake_rotation_deg: f64,
}

impl XTransform {
    const fn initial() -> Self {
        Self {
            translation_x: X_INITIAL_TRANSLATION_X,
            translation_y: X_INITIAL_TRANSLATION_Y,
            rotation_deg: X_INITIAL_ROTATION_DEG,
            scale: X_INITIAL_SCALE,
            brake_rotation_deg: 0.0,
        }
    }
}

impl Default for XTransform {
    fn default() -> Self {
        Self {
            translation_x: 0.0,
            translation_y: X_FINAL_Y_OFFSET_PX,
            rotation_deg: 0.0,
            scale: 1.0,
            brake_rotation_deg: 0.0,
        }
    }
}

impl Interpolatable for XTransform {}

fn banner_path(progress: f64, rustle_px: f64) -> Vec<PathElement> {
    let leading_x = lerp(BANNER_LEFT_X, BANNER_TIP_X, progress.max(0.002));
    let overshoot_x = (leading_x - BANNER_TIP_X).max(0.0);
    let shoulder_x = leading_x.min(BANNER_SHOULDER_X).max(BANNER_LEFT_X) + overshoot_x * 0.16;
    let top_y = BANNER_TOP_Y + rustle_px * 0.35;
    let tip_y = BANNER_TIP_Y + rustle_px;
    let bottom_y = BANNER_BOTTOM_Y - rustle_px * 0.25;

    vec![
        point_element(Point::new(BANNER_LEFT_X, BANNER_TOP_Y)),
        PathElement::Line,
        point_element(Point::new(shoulder_x, top_y)),
        PathElement::Line,
        point_element(Point::new(leading_x, tip_y)),
        PathElement::Line,
        point_element(Point::new(shoulder_x, bottom_y)),
        PathElement::Line,
        point_element(Point::new(BANNER_LEFT_X, BANNER_BOTTOM_Y)),
        PathElement::Close,
    ]
}

fn p_ring_path(transform: Transform) -> Vec<PathElement> {
    ellipse_path_about(
        P_RING_CENTER,
        P_RING_RADIUS_X_PX,
        P_RING_RADIUS_Y_PX,
        transform,
        P_COUNTER_CENTER,
    )
}

fn p_counter_path(transform: Transform) -> Vec<PathElement> {
    circle_path_about(
        P_COUNTER_CENTER,
        P_COUNTER_RADIUS_PX,
        transform,
        P_COUNTER_CENTER,
    )
}

fn p_reveal_mask_path() -> Vec<PathElement> {
    reveal_half_plane_path(BANNER_LEFT_X)
}

fn p_counter_reveal_mask_path(progress: f64) -> Vec<PathElement> {
    reveal_half_plane_path(lerp(BANNER_LEFT_X, 0.0, progress.clamp(0.0, 1.0)))
}

fn reveal_half_plane_path(boundary_x: f64) -> Vec<PathElement> {
    vec![
        point_element(Point::new(boundary_x, 0.0)),
        PathElement::Line,
        point_element(Point::new(CANVAS_WIDTH_PX, 0.0)),
        PathElement::Line,
        point_element(Point::new(CANVAS_WIDTH_PX, CANVAS_HEIGHT_PX)),
        PathElement::Line,
        point_element(Point::new(boundary_x, CANVAS_HEIGHT_PX)),
        PathElement::Close,
    ]
}

fn a_path(transform: Transform, morph_progress: f64) -> Vec<PathElement> {
    let morph_progress = morph_progress.clamp(0.0, 1.0);
    let kappa = 0.552_284_749_830_793_6;
    let center = A_CENTER;
    let radius = A_INITIAL_OUTER_RADIUS_PX;
    let left = Point::new(center.x - radius, center.y);
    let top = Point::new(center.x, center.y - radius);
    let right = Point::new(center.x + radius, center.y);
    let bottom = Point::new(center.x, center.y + radius);
    let left_top_control = Point::new(center.x - radius, center.y - radius * kappa);
    let top_left_control = Point::new(center.x - radius * kappa, center.y - radius);
    let top_right_control = Point::new(center.x + radius * kappa, center.y - radius);
    let right_top_control = Point::new(center.x + radius, center.y - radius * kappa);
    let right_bottom_control = Point::new(center.x + radius, center.y + radius * kappa);
    let bottom_right_control = Point::new(center.x + radius * kappa, center.y + radius);
    let bottom_left_control = Point::new(center.x - radius * kappa, center.y + radius);
    let left_bottom_control = Point::new(center.x - radius, center.y + radius * kappa);

    vec![
        transformed_point_element(
            morph_point(left, Point::new(297.93, 152.39), morph_progress),
            transform,
        ),
        transformed_cubic_element(
            morph_point(left_top_control, Point::new(306.34, 97.33), morph_progress),
            morph_point(top_left_control, Point::new(356.34, 55.03), morph_progress),
            transform,
        ),
        transformed_point_element(
            morph_point(top, Point::new(413.60, 55.03), morph_progress),
            transform,
        ),
        transformed_cubic_element(
            morph_point(top_right_control, Point::new(432.29, 55.03), morph_progress),
            morph_point(right_top_control, Point::new(450.09, 62.38), morph_progress),
            transform,
        ),
        transformed_point_element(
            morph_point(right, Point::new(465.49, 74.32), morph_progress),
            transform,
        ),
        PathElement::Line,
        transformed_point_element(
            morph_point(right, Point::new(465.49, 55.03), morph_progress),
            transform,
        ),
        PathElement::Line,
        transformed_point_element(
            morph_point(right, Point::new(530.43, 55.03), morph_progress),
            transform,
        ),
        PathElement::Line,
        transformed_point_element(
            morph_point(right, Point::new(530.43, 283.94), morph_progress),
            transform,
        ),
        PathElement::Line,
        transformed_point_element(
            morph_point(right, Point::new(465.49, 283.94), morph_progress),
            transform,
        ),
        PathElement::Line,
        transformed_point_element(
            morph_point(right, Point::new(465.49, 263.22), morph_progress),
            transform,
        ),
        transformed_cubic_element(
            morph_point(
                right_bottom_control,
                Point::new(452.01, 275.65),
                morph_progress,
            ),
            morph_point(
                bottom_right_control,
                Point::new(432.29, 282.51),
                morph_progress,
            ),
            transform,
        ),
        transformed_point_element(
            morph_point(bottom, Point::new(413.60, 282.51), morph_progress),
            transform,
        ),
        transformed_cubic_element(
            morph_point(
                bottom_left_control,
                Point::new(356.69, 282.51),
                morph_progress,
            ),
            morph_point(
                left_bottom_control,
                Point::new(306.95, 240.72),
                morph_progress,
            ),
            transform,
        ),
        transformed_point_element(
            morph_point(left, Point::new(298.08, 186.16), morph_progress),
            transform,
        ),
        transformed_cubic_element(
            morph_point(left, Point::new(295.40, 180.20), morph_progress),
            morph_point(left, Point::new(295.25, 158.20), morph_progress),
            transform,
        ),
        transformed_point_element(
            morph_point(left, Point::new(297.93, 152.39), morph_progress),
            transform,
        ),
        PathElement::Close,
    ]
}

fn a_counter_path(transform: Transform, morph_progress: f64) -> Vec<PathElement> {
    let morph_progress = morph_progress.clamp(0.0, 1.0);
    circle_path(
        morph_point(A_CENTER, A_COUNTER_CENTER, morph_progress),
        lerp(P_COUNTER_RADIUS_PX, A_COUNTER_RADIUS_PX, morph_progress),
        transform,
    )
}

fn a_counter_reveal_mask_path() -> Vec<PathElement> {
    let boundary_x = P_RING_CENTER.x + P_RING_RADIUS_X_PX;
    vec![
        point_element(Point::new(boundary_x, 0.0)),
        PathElement::Line,
        point_element(Point::new(CANVAS_WIDTH_PX, 0.0)),
        PathElement::Line,
        point_element(Point::new(CANVAS_WIDTH_PX, CANVAS_HEIGHT_PX)),
        PathElement::Line,
        point_element(Point::new(boundary_x, CANVAS_HEIGHT_PX)),
        PathElement::Close,
    ]
}

fn x_source_points() -> [Point; 13] {
    [
        Point::new(766.09, 282.99),
        Point::new(691.83, 282.99),
        Point::new(649.52, 220.99),
        Point::new(603.67, 282.99),
        Point::new(530.59, 282.99),
        Point::new(613.08, 169.71),
        Point::new(533.30, 54.52),
        Point::new(606.94, 54.52),
        Point::new(650.44, 117.45),
        Point::new(692.06, 54.52),
        Point::new(766.08, 54.52),
        Point::new(686.44, 169.71),
        Point::new(766.09, 282.99),
    ]
}

fn x_path(transform: XTransform) -> Vec<PathElement> {
    x_source_points()
        .into_iter()
        .enumerate()
        .flat_map(|(index, point)| {
            let point = transform_x_point(point, transform);
            if index == 0 {
                vec![point_element(point)]
            } else {
                vec![PathElement::Line, point_element(point)]
            }
        })
        .chain([PathElement::Close])
        .collect()
}

fn circle_path(center: Point, radius_px: f64, transform: Transform) -> Vec<PathElement> {
    circle_path_about(center, radius_px, transform, A_CENTER)
}

fn circle_path_about(
    center: Point,
    radius_px: f64,
    transform: Transform,
    pivot: Point,
) -> Vec<PathElement> {
    let kappa = 0.552_284_749_830_793_6;
    vec![
        transformed_point_element_about(
            Point::new(center.x + radius_px, center.y),
            transform,
            pivot,
        ),
        transformed_cubic_element_about(
            Point::new(center.x + radius_px, center.y + radius_px * kappa),
            Point::new(center.x + radius_px * kappa, center.y + radius_px),
            transform,
            pivot,
        ),
        transformed_point_element_about(
            Point::new(center.x, center.y + radius_px),
            transform,
            pivot,
        ),
        transformed_cubic_element_about(
            Point::new(center.x - radius_px * kappa, center.y + radius_px),
            Point::new(center.x - radius_px, center.y + radius_px * kappa),
            transform,
            pivot,
        ),
        transformed_point_element_about(
            Point::new(center.x - radius_px, center.y),
            transform,
            pivot,
        ),
        transformed_cubic_element_about(
            Point::new(center.x - radius_px, center.y - radius_px * kappa),
            Point::new(center.x - radius_px * kappa, center.y - radius_px),
            transform,
            pivot,
        ),
        transformed_point_element_about(
            Point::new(center.x, center.y - radius_px),
            transform,
            pivot,
        ),
        transformed_cubic_element_about(
            Point::new(center.x + radius_px * kappa, center.y - radius_px),
            Point::new(center.x + radius_px, center.y - radius_px * kappa),
            transform,
            pivot,
        ),
        transformed_point_element_about(
            Point::new(center.x + radius_px, center.y),
            transform,
            pivot,
        ),
        PathElement::Close,
    ]
}

fn ellipse_path_about(
    center: Point,
    radius_x_px: f64,
    radius_y_px: f64,
    transform: Transform,
    pivot: Point,
) -> Vec<PathElement> {
    let kappa = 0.552_284_749_830_793_6;
    vec![
        transformed_point_element_about(
            Point::new(center.x + radius_x_px, center.y),
            transform,
            pivot,
        ),
        transformed_cubic_element_about(
            Point::new(center.x + radius_x_px, center.y + radius_y_px * kappa),
            Point::new(center.x + radius_x_px * kappa, center.y + radius_y_px),
            transform,
            pivot,
        ),
        transformed_point_element_about(
            Point::new(center.x, center.y + radius_y_px),
            transform,
            pivot,
        ),
        transformed_cubic_element_about(
            Point::new(center.x - radius_x_px * kappa, center.y + radius_y_px),
            Point::new(center.x - radius_x_px, center.y + radius_y_px * kappa),
            transform,
            pivot,
        ),
        transformed_point_element_about(
            Point::new(center.x - radius_x_px, center.y),
            transform,
            pivot,
        ),
        transformed_cubic_element_about(
            Point::new(center.x - radius_x_px, center.y - radius_y_px * kappa),
            Point::new(center.x - radius_x_px * kappa, center.y - radius_y_px),
            transform,
            pivot,
        ),
        transformed_point_element_about(
            Point::new(center.x, center.y - radius_y_px),
            transform,
            pivot,
        ),
        transformed_cubic_element_about(
            Point::new(center.x + radius_x_px * kappa, center.y - radius_y_px),
            Point::new(center.x + radius_x_px, center.y - radius_y_px * kappa),
            transform,
            pivot,
        ),
        transformed_point_element_about(
            Point::new(center.x + radius_x_px, center.y),
            transform,
            pivot,
        ),
        PathElement::Close,
    ]
}

fn transformed_point_element(point: Point, transform: Transform) -> PathElement {
    transformed_point_element_about(point, transform, A_CENTER)
}

fn transformed_cubic_element(
    control_1: Point,
    control_2: Point,
    transform: Transform,
) -> PathElement {
    transformed_cubic_element_about(control_1, control_2, transform, A_CENTER)
}

fn transformed_point_element_about(
    point: Point,
    transform: Transform,
    pivot: Point,
) -> PathElement {
    point_element(transform_point_about(point, transform, pivot))
}

fn transformed_cubic_element_about(
    control_1: Point,
    control_2: Point,
    transform: Transform,
    pivot: Point,
) -> PathElement {
    cubic_element(
        transform_point_about(control_1, transform, pivot),
        transform_point_about(control_2, transform, pivot),
    )
}

fn transform_point_about(point: Point, transform: Transform, pivot: Point) -> Point {
    let radians = transform.rotation_deg.to_radians();
    let cosine = radians.cos();
    let sine = radians.sin();
    let local_x = (point.x - pivot.x) * transform.scale;
    let local_y = (point.y - pivot.y) * transform.scale;
    let primary = Point::new(
        pivot.x + local_x * cosine - local_y * sine + transform.translation_x,
        pivot.y + local_x * sine + local_y * cosine,
    );
    rotate_point_about(primary, A_BRAKE_PIVOT, transform.brake_rotation_deg)
}

fn transform_x_point(point: Point, transform: XTransform) -> Point {
    let radians = transform.rotation_deg.to_radians();
    let cosine = radians.cos();
    let sine = radians.sin();
    let local_x = (point.x - X_CENTER.x) * transform.scale;
    let local_y = (point.y - X_CENTER.y) * transform.scale;
    let primary = Point::new(
        X_CENTER.x + local_x * cosine - local_y * sine + transform.translation_x,
        X_CENTER.y + local_x * sine + local_y * cosine + transform.translation_y,
    );
    rotate_point_about(primary, X_BRAKE_PIVOT, transform.brake_rotation_deg)
}

fn rotate_point_about(point: Point, pivot: Point, rotation_deg: f64) -> Point {
    let radians = rotation_deg.to_radians();
    let cosine = radians.cos();
    let sine = radians.sin();
    let local_x = point.x - pivot.x;
    let local_y = point.y - pivot.y;
    Point::new(
        pivot.x + local_x * cosine - local_y * sine,
        pivot.y + local_x * sine + local_y * cosine,
    )
}

fn point_element(point: Point) -> PathElement {
    PathElement::Point(percent_x(point.x), percent_y(point.y))
}

fn cubic_element(control_1: Point, control_2: Point) -> PathElement {
    PathElement::Cubic(
        percent_x(control_1.x),
        percent_y(control_1.y),
        percent_x(control_2.x),
        percent_y(control_2.y),
    )
}

fn percent_x(value: f64) -> Size {
    percent(value / CANVAS_WIDTH_PX * 100.0)
}

fn percent_y(value: f64) -> Size {
    percent(value / CANVAS_HEIGHT_PX * 100.0)
}

fn percent(value: f64) -> Size {
    Size::Percent(Numeric::F64(value))
}

fn lerp(start: f64, end: f64, t: f64) -> f64 {
    start + (end - start) * t
}

fn morph_point(start: Point, end: Point, t: f64) -> Point {
    Point::new(lerp(start.x, end.x, t), lerp(start.y, end.y, t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_keeps_stable_topology_through_overshoot() {
        let lengths = [0.0, 0.25, 0.8, 1.0, 1.04]
            .map(|progress| banner_path(progress, 0.0))
            .map(|path| path.len());
        assert!(lengths.into_iter().all(|length| length == 10));
    }

    #[test]
    fn a_paths_keep_stable_topology_while_rolling() {
        for transform in [
            Transform {
                translation_x: A_INITIAL_TRANSLATION_X,
                rotation_deg: -90.0,
                scale: A_INITIAL_SCALE,
                brake_rotation_deg: 0.0,
            },
            Transform {
                translation_x: -110.0,
                rotation_deg: -45.0,
                scale: 0.6,
                brake_rotation_deg: 0.0,
            },
            Transform {
                translation_x: -5.0,
                rotation_deg: -2.0,
                scale: 0.98,
                brake_rotation_deg: 0.0,
            },
            Transform::default(),
        ] {
            for morph_progress in [0.0, 0.2, 0.75, 1.0] {
                assert_eq!(a_path(transform, morph_progress).len(), 22);
                assert_eq!(a_counter_path(transform, morph_progress).len(), 10);
            }
        }
    }

    #[test]
    fn settled_banner_and_letters_match_source_extents() {
        let banner = banner_path(1.0, 0.0);
        assert_eq!(banner[0], point_element(Point::new(137.23, 23.34)));
        assert_eq!(banner[4], point_element(Point::new(892.72, 168.57)));
        assert_eq!(banner[8], point_element(Point::new(137.23, 313.20)));

        let settled_a = a_path(Transform::default(), 1.0);
        assert_eq!(settled_a[0], point_element(Point::new(297.93, 152.39)));

        let settled_x = x_path(XTransform::default());
        assert_eq!(
            settled_x[0],
            point_element(Point::new(766.09, 282.99 + X_FINAL_Y_OFFSET_PX))
        );
    }

    #[test]
    fn p_counter_preserves_the_source_overlap_without_a_cutout_seam() {
        assert!(P_COUNTER_CENTER.x - P_COUNTER_RADIUS_PX < BANNER_LEFT_X);
    }

    #[test]
    fn p_counter_starts_fully_behind_the_pillar_edge() {
        let initial_right_edge = P_COUNTER_CENTER.x - 110.0 + P_COUNTER_RADIUS_PX * P_INITIAL_SCALE;
        assert!(initial_right_edge < BANNER_LEFT_X);
    }

    #[test]
    fn p_counter_reveal_mask_opens_after_emergence() {
        let hidden = p_counter_reveal_mask_path(0.0);
        let emerged = p_counter_reveal_mask_path(1.0);
        assert_eq!(hidden[0], point_element(Point::new(BANNER_LEFT_X, 0.0)));
        assert_eq!(emerged[0], point_element(Point::new(0.0, 0.0)));
    }

    #[test]
    fn p_ring_and_counter_keep_stable_topology_through_the_roll() {
        for transform in [
            Transform {
                translation_x: -110.0,
                rotation_deg: -90.0,
                scale: P_INITIAL_SCALE,
                brake_rotation_deg: 0.0,
            },
            Transform {
                translation_x: -45.0,
                rotation_deg: -35.0,
                scale: 0.72,
                brake_rotation_deg: 0.0,
            },
            Transform::default(),
        ] {
            assert_eq!(p_ring_path(transform).len(), 10);
            assert_eq!(p_counter_path(transform).len(), 10);
        }
    }

    #[test]
    fn a_morph_starts_as_a_circle_with_collapsed_sidebar_points() {
        let initial = a_path(
            Transform {
                translation_x: A_INITIAL_TRANSLATION_X,
                rotation_deg: -90.0,
                scale: A_INITIAL_SCALE,
                brake_rotation_deg: 0.0,
            },
            0.0,
        );
        assert_eq!(initial[4], initial[6]);
        assert_eq!(initial[6], initial[8]);
        assert_eq!(initial[8], initial[10]);
        assert_eq!(initial[10], initial[12]);
        assert_eq!(initial[12], initial[14]);
    }

    #[test]
    fn initial_a_stays_inside_the_p_counter_until_its_roll_begins() {
        let initial_center = Point::new(A_CENTER.x + A_INITIAL_TRANSLATION_X, A_CENTER.y);
        let dispatch_center = Point::new(
            P_COUNTER_CENTER.x + P_DISPATCH_TRANSLATION_X,
            P_COUNTER_CENTER.y,
        );
        let center_offset = ((initial_center.x - dispatch_center.x).powi(2)
            + (initial_center.y - P_COUNTER_CENTER.y).powi(2))
        .sqrt();
        assert!(
            center_offset + A_INITIAL_OUTER_RADIUS_PX * A_INITIAL_SCALE
                < P_COUNTER_RADIUS_PX * P_DISPATCH_SCALE
        );
    }

    #[test]
    fn x_starts_nested_inside_the_settled_a_sidebar() {
        let initial = XTransform::initial();
        let nested = x_source_points().map(|point| transform_x_point(point, initial));
        let min_x = nested
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min);
        let max_x = nested
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(min_x >= A_SIDEBAR_LEFT_X);
        assert!(max_x <= A_SIDEBAR_RIGHT_X);
    }

    #[test]
    fn braking_rotations_keep_the_lower_right_contacts_planted() {
        let a_contact = transform_point_about(
            A_BRAKE_PIVOT,
            Transform {
                brake_rotation_deg: 7.5,
                ..Transform::default()
            },
            A_CENTER,
        );
        assert!((a_contact.x - A_BRAKE_PIVOT.x).abs() < 0.000_001);
        assert!((a_contact.y - A_BRAKE_PIVOT.y).abs() < 0.000_001);

        let x_contact = transform_x_point(
            Point::new(766.09, 282.99),
            XTransform {
                brake_rotation_deg: 7.0,
                ..XTransform::default()
            },
        );
        assert!((x_contact.x - X_BRAKE_PIVOT.x).abs() < 0.000_001);
        assert!((x_contact.y - X_BRAKE_PIVOT.y).abs() < 0.000_001);
    }

    #[test]
    fn a_counter_stays_clipped_until_it_clears_the_p_ring() {
        let reveal_mask = a_counter_reveal_mask_path();
        let boundary_x = P_RING_CENTER.x + P_RING_RADIUS_X_PX;
        assert_eq!(reveal_mask[0], point_element(Point::new(boundary_x, 0.0)));
        assert_eq!(
            reveal_mask[6],
            point_element(Point::new(boundary_x, CANVAS_HEIGHT_PX))
        );
    }
}
