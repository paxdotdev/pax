use pax_engine::{
    api::{Numeric, Percent, Rotation, Window},
    math::Transform2,
    node_layout::{LayoutProperties, TransformAndBounds},
    NodeLocal,
};
use pax_std::Size;

use super::IntoDecompositionConfiguration;

#[test]
fn test_simple_units() {
    test_to_and_back(
        "origin",
        LayoutProperties {
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            ..Default::default()
        },
        TransformAndBounds::default(),
    );
    test_to_and_back(
        "pos_pixels",
        LayoutProperties {
            x: Some(Size::Pixels(50.into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            ..Default::default()
        },
        TransformAndBounds::default(),
    );
    test_to_and_back(
        "pos_percent",
        LayoutProperties {
            x: Some(Size::Percent(50.into())),
            width: Some(Size::Percent(500.into())),
            height: Some(Size::Percent(300.into())),
            ..Default::default()
        },
        TransformAndBounds::default(),
    );
}

#[test]
fn test_parent_transform() {
    let parent_transform = TransformAndBounds {
        transform: Transform2::new([1.0, 2.4, 3.2, -1.2, 5.0, 10.0]),
        bounds: (1000.0, 700.0),
    };
    test_to_and_back(
        "pos_pixels",
        LayoutProperties {
            y: Some(Size::Pixels((-50).into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "pos_percent",
        LayoutProperties {
            y: Some(Size::Percent(50.into())),
            width: Some(Size::Percent(500.into())),
            height: Some(Size::Percent(300.into())),
            ..Default::default()
        },
        parent_transform,
    );
}

#[test]
fn test_rotation() {
    let parent_transform = TransformAndBounds {
        transform: Transform2::new([1.0, 2.4, 3.2, 1.2, 5.0, 10.0]),
        bounds: (1000.0, 700.0),
    };
    test_to_and_back(
        "pos_pixels",
        LayoutProperties {
            x: Some(Size::Pixels(30.into())),
            y: Some(Size::Pixels((-20).into())),
            width: Some(Size::Pixels((500).into())),
            height: Some(Size::Pixels(300.into())),
            rotate: Some(Rotation::Degrees(35.0.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "pos_percent",
        LayoutProperties {
            x: Some(Size::Percent(30.into())),
            y: Some(Size::Percent((-500).into())),
            width: Some(Size::Percent(500.into())),
            height: Some(Size::Percent(350.into())),
            rotate: Some(Rotation::Degrees((-50.0).into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "pos_percent_and_pixels",
        LayoutProperties {
            x: Some(Size::Percent(30.into())),
            y: Some(Size::Pixels(50.into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Percent(300.into())),
            rotate: Some(Rotation::Degrees(34.1293.into())),
            ..Default::default()
        },
        parent_transform,
    );
}

#[test]
fn test_anchor() {
    let parent_transform = TransformAndBounds {
        transform: Transform2::new([1.0, 2.4, -3.2, 1.2, 5.0, 10.0]),
        bounds: (1000.0, 700.0),
    };
    test_to_and_back(
        "anchor_x",
        LayoutProperties {
            x: Some(Size::Pixels(30.into())),
            y: Some(Size::Percent(50.into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            rotate: Some(Rotation::Degrees(35.0.into())),
            anchor_x: Some(Size::Percent(50.0.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "anchor_y",
        LayoutProperties {
            x: Some(Size::Percent(30.into())),
            y: Some(Size::Percent(50.into())),
            width: Some(Size::Percent(500.into())),
            height: Some(Size::Percent(30.into())),
            rotate: Some(Rotation::Degrees((-50.0).into())),
            anchor_y: Some(Size::Pixels(1000.0.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "anchor_x_and_y",
        LayoutProperties {
            x: Some(Size::Pixels(30.into())),
            y: Some(Size::Percent((-300).into())),
            width: Some(Size::Pixels((100).into())),
            height: Some(Size::Pixels(300.into())),
            rotate: Some(Rotation::Degrees(73.2.into())),
            anchor_x: Some(Size::Percent(50.0.into())),
            anchor_y: Some(Size::Pixels(20.0.into())),
            ..Default::default()
        },
        parent_transform,
    );
}

#[test]
fn test_with_scale() {
    let parent_transform = TransformAndBounds {
        transform: Transform2::new([1.0, 2.4, 3.2, 1.2, 5.0, 10.0]),
        bounds: (100.0, 700.0),
    };
    test_to_and_back(
        "scale_x",
        LayoutProperties {
            x: Some(Size::Pixels(30.into())),
            y: Some(Size::Percent(50.into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            rotate: Some(Rotation::Degrees(35.0.into())),
            scale_x: Some(Percent(50.0.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "scale_y",
        LayoutProperties {
            x: Some(Size::Percent((-30).into())),
            y: Some(Size::Percent(50.into())),
            width: Some(Size::Percent(50.into())),
            height: Some(Size::Percent(30.into())),
            rotate: Some(Rotation::Degrees((12.0).into())),
            anchor_y: Some(Size::Pixels(100.0.into())),
            scale_y: Some(Percent(150.0.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "scale_x_and_y",
        LayoutProperties {
            x: Some(Size::Pixels((-3000).into())),
            y: Some(Size::Percent(50.into())),
            width: Some(Size::Pixels(1000.into())),
            height: Some(Size::Pixels(300.into())),
            rotate: Some(Rotation::Degrees(73.2.into())),
            anchor_x: Some(Size::Percent(50.0.into())),
            anchor_y: Some(Size::Pixels(20.0.into())),
            scale_x: Some(Percent(200.0.into())),
            scale_y: Some(Percent(10.0.into())),
            ..Default::default()
        },
        parent_transform,
    );
}

// NOTE: skew y is ignored, and should NOT be set. if set, the output
// layoutproperties "bakes" this in to the other properties, and returns skew_y = 0
#[test]
fn test_with_skew() {
    let parent_transform = TransformAndBounds {
        transform: Transform2::new([1.0, 2.3, 3.2, 7.2, 5.0, 9.0]),
        bounds: (904.0, 700.0),
    };
    test_to_and_back(
        "skew_no_rotate",
        LayoutProperties {
            x: Some(Size::Percent(30.into())),
            y: Some(Size::Pixels(50.into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.9.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "skew",
        LayoutProperties {
            x: Some(Size::Percent(30.into())),
            y: Some(Size::Percent(50.into())),
            width: Some(Size::Percent(500.into())),
            height: Some(Size::Percent(30.into())),
            rotate: Some(Rotation::Degrees((-50.0).into())),
            anchor_y: Some(Size::Pixels(1000.0.into())),
            skew_x: Some(Rotation::Degrees(55.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "anchor_x_and_y",
        LayoutProperties {
            x: Some(Size::Pixels(30.into())),
            y: Some(Size::Percent(50.into())),
            width: Some(Size::Percent(1000.into())),
            height: Some(Size::Pixels(300.into())),
            rotate: Some(Rotation::Degrees(73.2.into())),
            anchor_x: Some(Size::Percent(50.0.into())),
            anchor_y: Some(Size::Pixels(20.0.into())),
            ..Default::default()
        },
        parent_transform,
    );
}

#[test]
fn test_all_quadrants() {
    let parent_transform = TransformAndBounds {
        transform: Transform2::new([1.0, 2.3, 3.2, 7.2, 5.0, 9.0]),
        bounds: (1000.0, 1000.0),
    };
    test_to_and_back(
        "center",
        LayoutProperties {
            x: Some(Size::Percent(50.into())),
            y: Some(Size::Percent(50.into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.5.into())),
            rotate: Some(Rotation::Radians(0.5.into())),
            scale_y: Some(Percent(50.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "left",
        LayoutProperties {
            x: Some(Size::Percent((-500).into())),
            y: Some(Size::Percent(50.into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.5.into())),
            rotate: Some(Rotation::Radians(0.5.into())),
            scale_y: Some(Percent(50.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "right",
        LayoutProperties {
            x: Some(Size::Percent((500).into())),
            y: Some(Size::Percent(50.into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.5.into())),
            rotate: Some(Rotation::Radians(0.5.into())),
            scale_y: Some(Percent(50.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "top",
        LayoutProperties {
            x: Some(Size::Percent(50.into())),
            y: Some(Size::Percent((-500).into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.5.into())),
            rotate: Some(Rotation::Radians(0.5.into())),
            scale_y: Some(Percent(50.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "bottom",
        LayoutProperties {
            x: Some(Size::Percent(50.into())),
            y: Some(Size::Percent((500).into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.5.into())),
            rotate: Some(Rotation::Radians(0.5.into())),
            scale_y: Some(Percent(50.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "top_left",
        LayoutProperties {
            x: Some(Size::Percent((-600).into())),
            y: Some(Size::Percent((-500).into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.5.into())),
            rotate: Some(Rotation::Radians(0.5.into())),
            scale_y: Some(Percent(50.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "bottom_left",
        LayoutProperties {
            x: Some(Size::Percent((-600).into())),
            y: Some(Size::Percent((700).into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.5.into())),
            rotate: Some(Rotation::Radians(0.5.into())),
            scale_y: Some(Percent(50.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "top_right",
        LayoutProperties {
            x: Some(Size::Percent((600).into())),
            y: Some(Size::Percent((-700).into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.5.into())),
            rotate: Some(Rotation::Radians(0.5.into())),
            scale_y: Some(Percent(50.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "bottom_right",
        LayoutProperties {
            x: Some(Size::Percent((600).into())),
            y: Some(Size::Percent((-700).into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.5.into())),
            rotate: Some(Rotation::Radians((-0.5).into())),
            scale_y: Some(Percent((-50).into())),
            ..Default::default()
        },
        parent_transform,
    );
}

// NOTE: currently the positioning for an object that's larger than the parent
// WHILE ax/ay has a valid solution inside the boundary of the object is a bit
// funky. the solutions are valid BUT there might be many (possibly all 8 that
// are possible) could at some point more intelligently choose the solution
// instead of just take the first possible one (which is the interior),
// more complicated when an object is rotated (then some of the 8 solutions dissapear)
//
// What this means for this test method is that it might fail even when
// the bounding box of the returned solutions looks identical to the
// one created by the original properties.
//
// Add code marked with ">" to count number of solutions left in inversion impl:
//     ...
//     });
//     let (x, y) = solutions
//         .next()
//         .expect("transform inversion to common properties didn't find a solution");
//  >   let extra_solutions = solutions.count();
//  >   if extra_solutions > 0 {
//  >       log::warn!("found {} extra solutions!", extra_solutions);
//  >   }
//     (x, y)
// }
// // ax = w*x, ay fixed
// (None, Some(anchor_y)) => {
//     let ay = anchor_y.evaluate(object_bounds, Axis::Y);
//     let ax = w_r * (dx + M[0][1] * ay) / (1.0 - M[0][0] * w_r);
//    ...
#[test]
fn test_object_larger_than_parent() {
    let parent_transform = TransformAndBounds {
        transform: Transform2::new([1.0, 2.3, 3.2, 7.2, 5.0, 9.0]),
        bounds: (100.0, 100.0),
    };
    test_to_and_back(
        "center",
        LayoutProperties {
            x: Some(Size::Percent(50.into())),
            y: Some(Size::Percent(50.into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.5.into())),
            rotate: Some(Rotation::Radians(0.5.into())),
            scale_y: Some(Percent(50.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "right",
        LayoutProperties {
            x: Some(Size::Percent((500).into())),
            y: Some(Size::Percent(50.into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.5.into())),
            rotate: Some(Rotation::Radians(0.5.into())),
            scale_y: Some(Percent(50.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "top",
        LayoutProperties {
            x: Some(Size::Percent(50.into())),
            y: Some(Size::Percent((-500).into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.5.into())),
            rotate: Some(Rotation::Radians(0.5.into())),
            scale_y: Some(Percent(50.into())),
            ..Default::default()
        },
        parent_transform,
    );
    test_to_and_back(
        "bottom_left",
        LayoutProperties {
            x: Some(Size::Percent((-600).into())),
            y: Some(Size::Percent((700).into())),
            width: Some(Size::Pixels(500.into())),
            height: Some(Size::Pixels(300.into())),
            skew_x: Some(Rotation::Radians(0.5.into())),
            rotate: Some(Rotation::Radians(0.5.into())),
            scale_y: Some(Percent(50.into())),
            ..Default::default()
        },
        parent_transform,
    );
}

/// Helper method to test conversion to transform and back to layout props, and
/// then checking that the layout props are approximately the same
/// NOTE: If a test fails, that does NOT nessesarily mean that the returned
/// Layoutproperties don't visually cover the same area, but could be related to
/// HOW the solution is shosen. This can be verified visually by plugging in both
/// original_properties and recovered properties in the designer settings menu.
fn test_to_and_back(
    test_case_desc: &str,
    original_props: LayoutProperties,
    parent_transform_and_bounds: TransformAndBounds<NodeLocal, Window>,
) {
    let t_and_b = pax_engine::node_layout::calculate_transform_and_bounds(
        &original_props,
        parent_transform_and_bounds,
    );
    let inv_config = original_props.into_decomposition_config();
    let recovered_props = super::transform_and_bounds_decomposition(
        &inv_config,
        &parent_transform_and_bounds,
        &t_and_b,
    );

    fn print_failure_info<T: std::fmt::Debug>(
        info: &str,
        sym: &str,
        a: T,
        b: T,
        original_props: &LayoutProperties,
        recovered_props: &LayoutProperties,
    ) {
        panic!(
            "test {:?}: recovered layout_property {:?} had value {:?} which isn't close enough to {:?}\n\
            original_props: {:?}\n\
            recovered_props: {:?}",
            info, sym, b, a, original_props, recovered_props,
        );
    }

    let numerics_approx_eq = |a: &Numeric, b: &Numeric| (a.to_float() - b.to_float()).abs() < 1e-2;

    let assert_size_approx_eq = |sym, a: &Option<Size>, b: &Option<Size>, expected_fallback| {
        let a = &a.clone().or(b.is_some().then_some(expected_fallback));
        let are_approx_eq = match (a, b) {
            (Some(Size::Percent(a)), Some(Size::Percent(b))) => numerics_approx_eq(a, b),
            (Some(Size::Pixels(a)), Some(Size::Pixels(b))) => numerics_approx_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !are_approx_eq {
            print_failure_info(test_case_desc, sym, a, b, &original_props, &recovered_props)
        }
    };
    let assert_percent_approx_eq =
        |sym, a: &Option<Percent>, b: &Option<Percent>, expected_fallback| {
            let a = &a.clone().or(b.is_some().then_some(expected_fallback));
            let are_approx_eq = match (a, b) {
                (Some(Percent(a)), Some(Percent(b))) => numerics_approx_eq(a, b),
                (None, None) => true,
                _ => false,
            };
            if !are_approx_eq {
                print_failure_info(test_case_desc, sym, a, b, &original_props, &recovered_props)
            }
        };
    let assert_rotation_approx_eq = |sym,
                                     a: &Option<Rotation>,
                                     b: &Option<Rotation>,
                                     expected_fallback| {
        let a = &a.clone().or(b.is_some().then_some(expected_fallback));
        let are_approx_eq = match (a, b) {
            (Some(Rotation::Radians(a)), Some(Rotation::Radians(b))) => numerics_approx_eq(a, b),
            (Some(Rotation::Degrees(a)), Some(Rotation::Degrees(b))) => numerics_approx_eq(a, b),
            (Some(Rotation::Percent(a)), Some(Rotation::Percent(b))) => numerics_approx_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !are_approx_eq {
            print_failure_info(test_case_desc, sym, a, b, &original_props, &recovered_props)
        }
    };

    let LayoutProperties {
        x,
        y,
        width,
        height,
        rotate,
        scale_x,
        scale_y,
        anchor_x,
        anchor_y,
        skew_x,
        skew_y,
    } = &original_props;

    assert_size_approx_eq("x", &x, &recovered_props.x, Size::Percent(0.0.into()));
    assert_size_approx_eq("y", &y, &recovered_props.y, Size::Percent(0.0.into()));
    assert_rotation_approx_eq(
        "rotate",
        &rotate,
        &recovered_props.rotate,
        Rotation::Degrees(0.0.into()),
    );
    assert_size_approx_eq("with", &width, &recovered_props.width, Size::default());
    assert_size_approx_eq("height", &height, &recovered_props.height, Size::default());
    assert_percent_approx_eq(
        "scale_x",
        &scale_x,
        &recovered_props.scale_x,
        Percent(100.0.into()),
    );
    assert_percent_approx_eq(
        "scale_y",
        &scale_y,
        &recovered_props.scale_y,
        Percent(100.0.into()),
    );
    assert_size_approx_eq(
        "anchor_x",
        &anchor_x,
        &recovered_props.anchor_x,
        Size::default(),
    );
    assert_size_approx_eq(
        "anchor_y",
        &anchor_y,
        &recovered_props.anchor_y,
        Size::default(),
    );
    assert_rotation_approx_eq(
        "skew_x",
        &skew_x,
        &recovered_props.skew_x,
        Rotation::Degrees(0.0.into()),
    );
    assert_rotation_approx_eq(
        "skew_y",
        &skew_y,
        &recovered_props.skew_y,
        Rotation::Degrees(0.0.into()),
    );
}
