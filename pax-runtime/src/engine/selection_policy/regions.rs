use super::*;
use crate::engine::surface_preparation::{ObservedExposure, PreparationHost, PreparedHost};
use crate::engine::surface_scheduling::{CoverageRect, HostObservation};
use kurbo::Point;

pub(super) fn coverage(r: Rect) -> CoverageRect {
    CoverageRect {
        left: r.x0,
        top: r.y0,
        right: r.x1,
        bottom: r.y1,
    }
}
fn bounds(r: CoverageRect) -> Rect {
    Rect::new(r.left, r.top, r.right, r.bottom)
}
#[derive(Clone, Debug)]
pub(super) struct Motion {
    pub tier: Tier,
    pub vector: Vec2,
}
/// Permitted hypothetical local movements. Ancestor depth is immaterial when
/// its scroll does not need to move. Two independent hypotheses are the cold
/// speculation limit; observed simultaneous motion uses the separate exact sweep.
pub(super) fn motions(s: &Scene, h: &Host) -> Vec<Motion> {
    let active = s.active == Some(h.id);
    let moving = active && s.direction != [0, 0];
    let mut out = Vec::new();
    let mut add = |tier, local: [f64; 2]| {
        let v = h.project([h.clamp(0, local[0]), h.clamp(1, local[1])]);
        if v.hypot() > 1e-8 {
            out.push(Motion { tier, vector: v });
        }
    };
    if moving {
        add(
            Tier::Leading,
            [0, 1].map(|i| {
                f64::from(s.direction[i]) * (s.safety * h.viewport[i]).max(h.travel[i].abs())
            }),
        );
    }
    for i in 0..2 {
        for sign in [-1i8, 1] {
            let mut local = [0., 0.];
            let dir = s.direction[i];
            let tier = if moving && sign == dir {
                Tier::Leading
            } else if active {
                Tier::OtherWarm
            } else {
                Tier::NestedVisible
            };
            let scale = if moving && sign == -dir && dir != 0 {
                s.reverse
            } else {
                s.safety
            };
            local[i] = scale * h.viewport[i] * f64::from(sign);
            add(tier, local);
        }
    }
    if moving {
        let i = if s.direction[0] != 0
            && (s.direction[1] == 0 || h.travel[0].abs() > h.travel[1].abs())
        {
            0
        } else {
            1
        };
        for (tier, sign) in [(Tier::Forward, 1.), (Tier::Reverse, -1.)] {
            let mut local = [0., 0.];
            local[i] = sign * f64::from(s.direction[i]) * h.content[i];
            add(tier, local);
        }
    }
    out
}
#[derive(Clone, Debug, PartialEq)]
struct Poly(Vec<Point>);
fn cross(o: Point, a: Point, b: Point) -> f64 {
    (a - o).cross(b - o)
}
impl Poly {
    fn rectangle(r: Rect) -> Self {
        if !area(r) {
            return Self(Vec::new());
        }
        Self(vec![
            (r.x0, r.y0).into(),
            (r.x1, r.y0).into(),
            (r.x1, r.y1).into(),
            (r.x0, r.y1).into(),
        ])
    }
    fn bbox(&self) -> Rect {
        let p = self.0[0];
        self.0
            .iter()
            .fold(Rect::from_points(p, p), |r, &p| r.union_pt(p))
    }
    fn valid(&self) -> bool {
        self.0.len() >= 3
            && (1..self.0.len() - 1)
                .map(|i| cross(self.0[0], self.0[i], self.0[i + 1]))
                .sum::<f64>()
                .abs()
                > 1e-7
    }
    fn shifted(&self, v: Vec2) -> Self {
        Self(self.0.iter().map(|p| *p + v).collect())
    }
    fn swept(&self, v: Vec2) -> Self {
        let mut pts = self.0.clone();
        pts.extend(self.0.iter().map(|p| *p + v));
        pts.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        pts.dedup();
        let half = |it: Vec<Point>| {
            let mut h = Vec::new();
            for p in it {
                while h.len() >= 2 && cross(h[h.len() - 2], h[h.len() - 1], p) <= 1e-9 {
                    h.pop();
                }
                h.push(p);
            }
            h.pop();
            h
        };
        let mut h = half(pts.clone());
        pts.reverse();
        h.extend(half(pts));
        Self(h)
    }
    fn clip(&self, r: Rect) -> Self {
        if !area(r) || self.0.is_empty() || !area(self.bbox().intersect(r)) {
            return Self(Vec::new());
        }
        let mut v = self.0.clone();
        for (axis, side, sign) in [(0, r.x0, 1.), (0, r.x1, -1.), (1, r.y0, 1.), (1, r.y1, -1.)] {
            if v.is_empty() {
                break;
            }
            let coord = |p: Point| ((if axis == 0 { p.x } else { p.y }) - side) * sign;
            let mut out = Vec::new();
            let mut p = *v.last().unwrap();
            let mut u = coord(p);
            for &q in &v {
                let t = coord(q);
                if (u >= 0.) != (t >= 0.) {
                    out.push(p + (q - p) * (u / (u - t)));
                }
                if t >= 0. {
                    out.push(q);
                }
                p = q;
                u = t;
            }
            v = out;
        }
        Self(v)
    }
}
#[derive(Clone, Debug)]
struct Zone {
    tier: Tier,
    steps: u8,
    poly: Poly,
}
pub(super) struct Plan {
    zones: Vec<Zone>,
    sweep: Option<ObservedExposure>,
    pub bias: Vec2,
    pub origin: Rect,
    pub near: Vec<(Tier, Rect)>,
    visible: Option<Rect>,
    pub far: [Option<Rect>; 2],
}
impl Plan {
    pub fn build(s: &Scene, domain: Domain) -> Self {
        let bias = s.bias(domain);
        let mut plan = Self {
            zones: Vec::new(),
            sweep: None,
            bias,
            origin: s.viewport + bias,
            near: vec![],
            visible: None,
            far: [None, None],
        };
        let Some(chain) = s.chain(domain) else {
            return plan;
        };
        let mut zones = vec![Zone {
            tier: Tier::Visible,
            steps: 0,
            poly: Poly::rectangle(s.viewport),
        }];
        for h in &chain {
            let aperture = h.clip.map_or(h.bounds, |clip| h.bounds.intersect(clip));
            let moves = motions(s, h);
            let mut next = Vec::new();
            for z in zones {
                let poly = z.poly.clip(aperture);
                if !poly.valid() {
                    continue;
                }
                if z.steps < 2 && z.tier.near() {
                    for m in &moves {
                        if z.steps > 0 && !m.tier.near() {
                            continue;
                        }
                        let tier = if z.steps == 0 {
                            m.tier
                        } else {
                            Tier::NestedWarm
                        };
                        next.push(Zone {
                            tier,
                            steps: z.steps + 1,
                            poly: poly.swept(m.vector),
                        });
                    }
                }
                next.push(Zone { poly, ..z });
            }
            zones = next;
        }
        if chain.iter().any(|h| h.travel != [0., 0.]) {
            let hosts: Vec<_> = chain
                .iter()
                .map(|h| PreparationHost {
                    geometry: HostObservation {
                        id: h.id,
                        parent: h.parent,
                        bounds: coverage(h.bounds),
                        visible: coverage(h.bounds),
                        viewport: h.viewport,
                        offset: h.offset,
                        basis: h.basis,
                        projection: None,
                        active: true,
                    },
                    content: h.content,
                    content_origin: h.origin,
                    intermediate_clip: h.clip.map(coverage),
                })
                .collect();
            let travel: Vec<_> = chain
                .iter()
                .map(|h| [0, 1].map(|i| h.clamp(i, h.travel[i])))
                .collect();
            let sweep = PreparedHost::observed_exposure(
                &hosts.iter().collect::<Vec<_>>(),
                coverage(s.viewport),
                &travel,
            );
            let b = bounds(sweep.bounds) + bias;
            if area(b) {
                plan.near.push((Tier::Leading, b));
                plan.sweep = Some(sweep);
            }
        }
        zones.sort_by_key(|z| z.tier);
        for mut z in zones {
            z.poly = z.poly.shifted(bias);
            let b = z.poly.bbox();
            if z.tier == Tier::Visible {
                plan.origin = b;
                plan.visible = Some(b);
            }
            if z.tier.near() {
                plan.near.push((z.tier, b))
            } else {
                let i = usize::from(z.tier == Tier::Reverse);
                plan.far[i] = Some(plan.far[i].map_or(b, |r| r.union(b)));
            }
            plan.zones.push(z);
        }
        plan
    }
    pub fn possible(&self, r: Option<Rect>) -> Tier {
        r.and_then(|r| {
            self.near
                .iter()
                .filter(|(_, b)| area(b.intersect(r)))
                .map(|(t, _)| *t)
                .min()
        })
        .unwrap_or(Tier::Forward)
    }
    pub fn tier(&self, r: Rect) -> Tier {
        if self.visible.is_some_and(|v| area(v.intersect(r))) {
            return Tier::Visible;
        }
        let mut tier = self
            .zones
            .iter()
            .find(|z| z.poly.clip(r).valid())
            .map_or(Tier::Remote, |z| z.tier);
        if tier > Tier::Leading
            && self
                .sweep
                .as_ref()
                .is_some_and(|s| s.intersects(coverage(r - self.bias)))
        {
            tier = Tier::Leading
        }
        tier
    }
}
