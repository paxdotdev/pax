use crate::rotating_headline::HERO_TIMING_SCALE;
use crate::SiteTheme;
use pax_kit::*;

/// One shaped cluster: ligatures and combining marks move as a single unit.
/// Coordinates are em fractions; geometry is independent of animation and size.
#[pax]
pub struct HeroCluster {
    pub byte_start: usize,
    pub byte_end: usize,
    pub ordinal: usize,
    pub glyph_ids: Vec<u32>,
    pub x: f64,
    pub advance: f64,
    pub elements: Vec<PathElement>,
}

#[pax]
#[file("hero_letter.pax")]
pub struct HeroLetter {
    pub cluster: Property<HeroCluster>,
    pub ordinal: Property<usize>,
    pub font_px: Property<f64>,
    pub cap_height: Property<f64>,
    pub time_ms: Property<f64>,
    pub kind: Property<usize>,
    pub dx: Property<f64>,
    pub dy: Property<f64>,
    pub sx: Property<f64>,
    pub sy: Property<f64>,
    pub angle: Property<f64>,
    pub alpha: Property<f64>,
    pub paint: Property<Paint>,
    pub material: Property<Material>,
}

impl HeroLetter {
    pub fn mount(&mut self, ctx: &NodeContext) {
        // Reused keyed clusters can change word/color at a handoff. Paint depends
        // only on identity; motion never reconstructs their paths.
        let kind = self.kind.local();
        let ordinal = self.ordinal.local();
        let deps = [kind.untyped(), ordinal.untyped()];
        self.paint.replace_with(LocalProperty::computed(
            move || letter_paint(kind.get(), ordinal.get()),
            &deps,
        ));
        let kind = self.kind.local();
        let deps = [kind.untyped()];
        self.material.replace_with(LocalProperty::computed(
            move || {
                if kind.get() == 1 || kind.get() == 2 {
                    // Lift this material's ambient response, without adding a
                    // layer-wide AmbientLight that would change nearby demos.
                    Material::custom(MaterialParams {
                        ambient: Property::new(2.6),
                        diffuse: Property::new(0.18),
                        specular: Property::new(0.62),
                        roughness: Property::new(0.18),
                        metallic: Property::new(0.82),
                        ..Default::default()
                    })
                } else {
                    Material::unlit()
                }
            },
            &deps,
        ));
        self.tick(ctx);
    }

    pub fn tick(&mut self, _ctx: &NodeContext) {
        let pose = letter_pose(self.kind.get(), self.ordinal.get(), self.time_ms.get());
        self.dx.set_if_neq(pose.x);
        self.dy.set_if_neq(pose.y);
        self.sx.set_if_neq(pose.sx * 100.0);
        self.sy.set_if_neq(pose.sy * 100.0);
        self.angle.set_if_neq(pose.angle);
        self.alpha.set_if_neq(pose.alpha);
    }
}

fn letter_paint(kind: usize, index: usize) -> Paint {
    if kind != 1 && kind != 2 {
        return Paint::Solid(SiteTheme::paper());
    }
    let base = if kind == 1 {
        Color::from_rgba_0_1([0.76, 0.81, 0.88, 1.0])
    } else {
        // Champagne, copper, rose, violet titanium, cobalt, and oxidized teal.
        let colors = [
            (0.88, 0.72, 0.39),
            (0.88, 0.49, 0.29),
            (0.88, 0.48, 0.61),
            (0.66, 0.53, 0.90),
            (0.36, 0.64, 0.94),
            (0.33, 0.80, 0.69),
        ];
        let (r, g, b) = colors[index % colors.len()];
        Color::from_rgba_0_1([r, g, b, 1.0])
    };
    // Narrow opposing value bands suggest polished metal even without GPU lights.
    let stops = [
        (0.0, 0.64),
        (0.24, 0.95),
        (0.43, 1.0),
        (0.49, 0.50),
        (0.58, 0.72),
        (0.83, 0.92),
        (1.0, 0.68),
    ]
    .into_iter()
    .map(|(p, brightness)| {
        let [r, g, b, _] = base.to_rgba_0_1();
        GradientStop {
            position: Size::Percent((p * 100.0).into()),
            color: Color::from_rgba_0_1([r * brightness, g * brightness, b * brightness, 1.0]),
        }
    })
    .collect();
    Paint::LinearGradient(LinearGradient {
        start: (Size::Percent(0.0.into()), Size::Percent(0.0.into())),
        end: (Size::Percent(0.0.into()), Size::Percent(100.0.into())),
        stops,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LetterPose {
    x: f64,
    y: f64,
    sx: f64,
    sy: f64,
    angle: f64,
    alpha: f64,
}
const REST: LetterPose = LetterPose {
    x: 0.0,
    y: 0.0,
    sx: 1.0,
    sy: 1.0,
    angle: 0.0,
    alpha: 1.0,
};

fn smooth(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn letter_pose(kind: usize, index: usize, time: f64) -> LetterPose {
    let time = time / HERO_TIMING_SCALE;
    let delay = index as f64
        * match kind {
            1 => 23.0,
            2 => 85.0,
            _ => 58.0,
        };
    let duration = match kind {
        1 => 560.0,
        2 => 650.0,
        _ => 720.0,
    };
    let t = ((time - delay) / duration).clamp(0.0, 1.0);
    if t >= 1.0 {
        return REST;
    }
    let mut p = REST;
    p.alpha = smooth(t / 0.22);
    match kind {
        1 => {
            // Build speed, brake just beyond the stop, then make one precise correction.
            p.x = if t < 0.62 {
                1.65 + (-1.72) * smooth(t / 0.62)
            } else {
                -0.07 * (1.0 - smooth((t - 0.62) / 0.38))
            };
        }
        2 => {
            // A traveling press into the baseline, followed by a restrained release.
            let press = if t < 0.34 {
                smooth(t / 0.34)
            } else if t < 0.65 {
                1.0 - 1.24 * smooth((t - 0.34) / 0.31)
            } else {
                -0.24 * (1.0 - smooth((t - 0.65) / 0.35))
            };
            p.sy = 1.0 - 0.23 * press;
            p.sx = 1.0 + 0.06 * press;
            p.y = 0.07 * press;
        }
        _ => {
            let remaining = 1.0 - smooth(t);
            let side = if index % 2 == 0 { -1.0 } else { 1.0 };
            p.x = side * 0.28 * remaining;
            p.y = -0.42 * remaining - 0.14 * (std::f64::consts::PI * t).sin();
            p.angle = side * 13.0 * remaining;
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_letters_settle_and_pause_pose_is_exact() {
        for kind in 1..=3 {
            for index in 0..16 {
                assert_eq!(letter_pose(kind, index, 2500.0), REST);
                assert_eq!(letter_pose(kind, index, 0.0).alpha, 0.0);
                for time in (0..2500).step_by(13) {
                    let p = letter_pose(kind, index, time as f64);
                    assert!(p.sx > 0.0 && p.sy > 0.0 && (0.0..=1.0).contains(&p.alpha));
                }
            }
        }
        assert!(letter_pose(1, 0, 400.0 * HERO_TIMING_SCALE).x < 0.0);
        assert!(letter_pose(2, 0, 230.0 * HERO_TIMING_SCALE).sy < 1.0);
        assert!(letter_pose(2, 0, 500.0 * HERO_TIMING_SCALE).sy > 1.0);
        assert!(letter_pose(3, 0, 180.0).angle < 0.0);
        assert!(letter_pose(3, 1, 180.0).angle > 0.0);
    }
}
