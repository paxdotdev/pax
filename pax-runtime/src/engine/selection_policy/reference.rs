//! Enumerate legal work and solve visibility in offset-parameter space. This
//! deliberately does not use the spatial index, its regions or PreparedHost.
use super::*;
use kurbo::Point;
use std::collections::BTreeMap;

// Derive the allowed local displacements directly from the recorded ranges.
// The only shared inputs with the indexed classifier are geometry and policy.
struct RawMove {
    tier: Tier,
    vector: Vec2,
}
fn raw_moves(scene: &Scene, h: &Host) -> Vec<RawMove> {
    let active = scene.active == Some(h.id);
    let heading = if active { scene.direction } else { [0, 0] };
    let mut moves = Vec::new();
    let mut add = |tier, request: [f64; 2]| {
        let mut d = [0.; 2];
        for i in 0..2 {
            let low = h.origin[i];
            let high = low + (h.content[i] - h.viewport[i]).max(0.);
            let start = h.offset[i].max(low).min(high);
            d[i] = (start + request[i]).max(low).min(high) - start;
        }
        let [xx, xy, yx, yy] = h.basis;
        let vector = Vec2::new(xx * d[0] + yx * d[1], xy * d[0] + yy * d[1]);
        if vector.hypot() > 1e-8 {
            moves.push(RawMove { tier, vector });
        }
    };
    for i in 0..2 {
        for sign in [-1i8, 1] {
            let in_front = heading[i] == sign;
            let scale = if heading[i] == -sign {
                scene.reverse
            } else {
                scene.safety
            };
            let mut displacement = [0.; 2];
            displacement[i] = f64::from(sign) * h.viewport[i] * scale;
            add(
                if in_front {
                    Tier::Leading
                } else if active {
                    Tier::OtherWarm
                } else {
                    Tier::NestedVisible
                },
                displacement,
            );
        }
    }
    if heading != [0, 0] {
        add(
            Tier::Leading,
            [0, 1].map(|i| {
                f64::from(heading[i]) * h.travel[i].abs().max(h.viewport[i] * scene.safety)
            }),
        );
        let axis = match heading {
            [0, _] => 1,
            [_, 0] => 0,
            _ => usize::from(h.travel[1].abs() >= h.travel[0].abs()),
        };
        let mut d = [0.; 2];
        d[axis] = f64::from(heading[axis]) * h.content[axis];
        add(Tier::Forward, d);
        add(Tier::Reverse, d.map(|v| -v));
    }
    moves
}
#[derive(Clone, Copy)]
struct Window {
    r: Rect,
    v: [Vec2; 2],
}
#[derive(Clone, Copy)]
struct Limit(f64, f64, f64);
fn limits(a: Window, b: Window) -> [Limit; 4] {
    let [x, y] = [a.v[0] - b.v[0], a.v[1] - b.v[1]];
    [
        Limit(x.x, y.x, b.r.x1 - a.r.x0),
        Limit(-x.x, -y.x, a.r.x1 - b.r.x0),
        Limit(x.y, y.y, b.r.y1 - a.r.y0),
        Limit(-x.y, -y.y, a.r.y1 - b.r.y0),
    ]
}
#[derive(Clone)]
enum Feasible {
    AtRest,
    Interval(f64, f64),
    Polygon(Vec<Point>),
}
impl Feasible {
    fn restrict(&mut self, Limit(x, y, r): Limit) -> bool {
        let r = r - 1e-8;
        if x.abs() + y.abs() < 1e-12 {
            return r >= 0.;
        }
        match self {
            Self::AtRest => r >= 0.,
            Self::Interval(lo, hi) => {
                if x > 0. {
                    *hi = hi.min(r / x)
                } else {
                    *lo = lo.max(r / x)
                }
                *lo <= *hi
            }
            Self::Polygon(v) => {
                let Some(&mut_prev) = v.last() else {
                    return false;
                };
                let mut p = mut_prev;
                let mut prev = r - x * p.x - y * p.y;
                let mut out = Vec::new();
                for &q in v.iter() {
                    let now = r - x * q.x - y * q.y;
                    if (prev >= 0.) != (now >= 0.) {
                        out.push(p + (q - p) * (prev / (prev - now)));
                    }
                    if now >= 0. {
                        out.push(q);
                    }
                    p = q;
                    prev = now;
                }
                *v = out;
                !v.is_empty()
            }
        }
    }
}
struct Route {
    tier: Tier,
    w: Vec<Window>,
    valid: Feasible,
    bounds: Rect,
}
impl Route {
    fn new(
        scene: &Scene,
        chain: &[&Host],
        steps: &[(usize, usize, Vec2)],
        tier: Tier,
    ) -> Option<Self> {
        let mut v = [Vec2::ZERO; 2];
        for &(_, axis, d) in steps {
            v[axis] += d;
        }
        let mut w = vec![Window {
            r: scene.viewport,
            v,
        }];
        for (i, h) in chain.iter().enumerate() {
            w.push(Window { r: h.bounds, v });
            if let Some(r) = h.clip {
                w.push(Window { r, v });
            }
            for &(_, axis, d) in steps.iter().filter(|(n, _, _)| *n == i) {
                v[axis] -= d;
            }
        }
        let mut valid = if steps.is_empty() {
            Feasible::AtRest
        } else if steps.iter().any(|(_, i, _)| *i == 1) {
            Feasible::Polygon(vec![
                (0., 0.).into(),
                (1., 0.).into(),
                (1., 1.).into(),
                (0., 1.).into(),
            ])
        } else {
            Feasible::Interval(0., 1.)
        };
        let mut coarse = None::<Rect>;
        for (i, a) in w.iter().enumerate() {
            if !area(a.r) {
                return None;
            }
            let box_ =
                a.r.union(a.r + a.v[0])
                    .union(a.r + a.v[1])
                    .union(a.r + a.v[0] + a.v[1]);
            coarse = Some(coarse.map_or(box_, |r| r.intersect(box_)));
            for b in &w[..i] {
                if !limits(*a, *b).into_iter().all(|l| valid.restrict(l)) {
                    return None;
                }
            }
        }
        let bounds = coarse.unwrap();
        area(bounds).then_some(Self {
            tier,
            w,
            valid,
            bounds,
        })
    }
    fn intersects(&self, r: Rect) -> bool {
        if !area(r.intersect(self.bounds)) {
            return false;
        }
        if matches!(self.valid, Feasible::AtRest) {
            return true;
        }
        let t = Window {
            r,
            v: [Vec2::ZERO; 2],
        };
        let mut feasible = self.valid.clone();
        self.w
            .iter()
            .all(|&a| limits(a, t).into_iter().all(|l| feasible.restrict(l)))
    }
}
struct DomainRoutes {
    routes: Vec<Route>,
    origin: Rect,
    bias: Vec2,
}
impl DomainRoutes {
    fn from_raw(scene: &Scene, domain: Domain) -> Self {
        let mut routes = Vec::new();
        let mut origin = scene.viewport;
        if let Some(chain) = scene.chain(domain) {
            if let Some(r) = Route::new(scene, &chain, &[], Tier::Visible) {
                origin = r.bounds;
                routes.push(r);
            }
            let observed: Vec<_> = chain
                .iter()
                .enumerate()
                .filter(|(_, h)| h.travel != [0., 0.])
                .map(|(i, h)| {
                    (
                        i,
                        0,
                        h.project([h.clamp(0, h.travel[0]), h.clamp(1, h.travel[1])]),
                    )
                })
                .collect();
            if !observed.is_empty() {
                routes.extend(Route::new(scene, &chain, &observed, Tier::Leading));
            }
            let moves: Vec<_> = chain.iter().map(|h| raw_moves(scene, h)).collect();
            for (i, ms) in moves.iter().enumerate() {
                for m in ms {
                    routes.extend(Route::new(scene, &chain, &[(i, 0, m.vector)], m.tier));
                    if !m.tier.near() {
                        continue;
                    }
                    for (j, ns) in moves.iter().enumerate().skip(i + 1) {
                        for n in ns.iter().filter(|n| n.tier.near()) {
                            routes.extend(Route::new(
                                scene,
                                &chain,
                                &[(i, 0, m.vector), (j, 1, n.vector)],
                                Tier::NestedWarm,
                            ));
                        }
                    }
                }
            }
        }
        routes.sort_by_key(|r| r.tier);
        Self {
            routes,
            origin,
            bias: scene.bias(domain),
        }
    }
}
pub(super) struct Reference<'a> {
    scene: &'a Scene,
    domains: BTreeMap<Domain, DomainRoutes>,
}
impl<'a> Reference<'a> {
    pub fn new(scene: &'a Scene) -> Self {
        Self {
            scene,
            domains: Default::default(),
        }
    }
    pub fn owner(&mut self, o: &Owner) -> (Tier, u64, Domain) {
        let Some(b) = o.bounds.filter(|b| area(*b)) else {
            return (Tier::Remote, u64::MAX, o.domain);
        };
        let routes = self
            .domains
            .entry(o.domain)
            .or_insert_with(|| DomainRoutes::from_raw(self.scene, o.domain));
        let screen = b - routes.bias;
        let tier = routes
            .routes
            .iter()
            .find(|r| r.intersects(screen))
            .map_or(Tier::Remote, |r| r.tier);
        (tier, px_key(distance(routes.origin, screen)), o.domain)
    }
    pub fn enumerate(
        &mut self,
        jobs: &BTreeMap<Id, Job>,
        owners: &BTreeMap<Id, Owner>,
        resource: Resource,
        s: &Service,
    ) -> BTreeMap<Id, Choice> {
        let mut ranks = BTreeMap::new();
        jobs.iter()
            .filter(|(_, j)| j.resource == resource && j.status == Status::Runnable)
            .filter_map(|(&id, j)| {
                j.owners
                    .iter()
                    .filter_map(|oid| {
                        owners
                            .get(oid)
                            .map(|o| *ranks.entry(*oid).or_insert_with(|| self.owner(o)))
                    })
                    .map(|(tier, distance, host)| Choice {
                        id,
                        age: j.age,
                        tier,
                        distance,
                        host,
                    })
                    .min_by_key(|c| s.order(*c))
                    .map(|c| (id, c))
            })
            .collect()
    }
}
