#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

use core::f64::consts::PI;
use pax_engine::math::Generic;
use pax_engine::math::Point2;

#[pax]
#[custom(Default)]
#[file("path_animation.pax")]
pub struct PathAnimation {
    pub t: Property<Numeric>,
    pub resolution: Property<Numeric>,
    pub path_config: Property<PathConfig>,
    pub fill: Property<Color>,

    //private path elements
    pub path_elements: Property<Vec<PathElement>>,
}

impl Default for PathAnimation {
    fn default() -> Self {
        let fill = Property::new(Color::RED);
        let t: Property<Numeric> = Property::new(0.0.into());
        let resolution: Property<Numeric> = Property::new(60.into());
        let path_config = Property::new(PathConfig {
            amplitude: Property::new(0.3.into()),
            amplitude_ramp: Property::new(0.3.into()),
            frequency: Property::new(1.0.into()),
            frequency_ramp: Property::new(1.0.into()),
            thickness: Property::new(0.01.into()),
            thickness_ramp: Property::new(0.3.into()),
            span: Property::new(0.3.into()),
        });

        let path_elements = {
            let t = t.clone();
            let resolution = resolution.clone();
            let path_config = path_config.clone();
            let deps = [t.untyped(), resolution.untyped(), path_config.untyped()];
            Property::computed(
                move || {
                    let profile = |t: f64| t * (t - 1.0);
                    let conf = path_config.get();
                    let path = |t: f64| PathPoint {
                        point: Point2::<Generic>::new(
                            t,
                            0.5 + (conf.amplitude.get().to_float()
                                + conf.amplitude_ramp.get().to_float() * t)
                                * ((conf.frequency.get().to_float()
                                    + conf.frequency_ramp.get().to_float() * t)
                                    * t
                                    * PI
                                    * 2.0)
                                    .sin(),
                        ),
                        thickness: conf.thickness.get().to_float()
                            + conf.thickness_ramp.get().to_float() * t,
                    };
                    parametric_path(
                        resolution.get().to_int() as usize,
                        path,
                        profile,
                        conf.span.get().to_float(),
                        t.get().to_float(),
                    )
                },
                &deps,
            )
        };

        Self {
            fill,
            path_config,
            t,
            resolution,
            path_elements,
        }
    }
}

#[pax]
pub struct PathConfig {
    pub amplitude: Property<Numeric>,
    pub amplitude_ramp: Property<Numeric>,
    pub frequency: Property<Numeric>,
    pub frequency_ramp: Property<Numeric>,
    pub thickness: Property<Numeric>,
    pub thickness_ramp: Property<Numeric>,
    pub span: Property<Numeric>,
}

struct PathPoint {
    point: Point2,
    thickness: f64,
}
/// Takes a parametric function that describes a path:
/// path_points: parametric value (0.0 to 1.0) -> point on path + thikness modifier
/// path_profile: parametric value (0.0 to 1.0) -> thickness at point
/// span: how large part of the path is drawn at any one time?
/// returns: a function that given a time (0.0 to 1.0), gives back a path
fn parametric_path(
    resolution: usize,
    path_points: impl Fn(f64) -> PathPoint,
    path_profile: impl Fn(f64) -> f64,
    span: f64,
    t: f64,
) -> Vec<PathElement> {
    let r90 = Rotation::Degrees(90.into());
    let ran = 0..=resolution;
    let l: Vec<_> = ran
        .map(|v| {
            let s = v as f64 / resolution as f64; //0.0 to 1.0
            let mut p = path_points(s * span + (1.0 - span) * t);
            p.thickness *= path_profile(s);
            p
        })
        .collect();
    let mut normals = Vec::with_capacity(l.len());
    for i in 0..l.len() {
        normals.push(
            match (l.get(i.overflowing_sub(1).0), l.get(i), l.get(i + 1)) {
                (Some(a), Some(b), Some(c)) => {
                    (a.point.lerp_towards(b.point, 0.95) - c.point.lerp_towards(b.point, 0.95))
                        .rotate(r90.clone())
                        .normalize()
                        * b.thickness
                }
                (Some(a), Some(b), None) => {
                    (a.point - b.point).rotate(r90.clone()).normalize() * b.thickness
                }
                (None, Some(a), Some(b)) => {
                    (a.point - b.point).rotate(r90.clone()).normalize() * a.thickness
                }
                _ => panic!("unexpected"),
            },
        );
    }

    // adding/subtracting normals from path points give two lines offset by the same amount from the center
    let top = l.iter().zip(normals.iter()).map(|(a, &b)| a.point + b);
    let bottom = l.iter().zip(normals.iter()).map(|(a, &b)| a.point - b);

    // Now join top and bottom to create pathelements
    let mut elements: Vec<_> = top
        .chain(bottom.rev())
        .flat_map(|p| {
            [
                PathElement::point(
                    Size::Percent((100.0 * p.x).into()),
                    Size::Percent((100.0 * p.y).into()),
                ),
                PathElement::line(),
            ]
        })
        .collect();
    // remove last line
    elements.pop();
    // instead close the loop
    elements.push(PathElement::close());
    elements
}
