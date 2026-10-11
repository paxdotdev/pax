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
}

impl HeroLetter {
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
        assert!(letter_pose(1, 0, 400.0).x < 0.0);
        assert!(letter_pose(2, 0, 230.0).sy < 1.0);
        assert!(letter_pose(2, 0, 500.0).sy > 1.0);
        assert!(letter_pose(3, 0, 180.0).angle < 0.0);
        assert!(letter_pose(3, 1, 180.0).angle > 0.0);
    }
}
