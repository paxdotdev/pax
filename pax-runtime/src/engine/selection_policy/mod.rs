//! Proposed scroll admission policy. Checked replay consumes immutable geometry
//! and operation deltas. Execution and presentation remain with their owners.
mod activity;
mod indexed;
pub(in crate::engine) mod live;
mod reference;
mod regions;
#[cfg(any(test, feature = "scroll-selection-shadow"))]
mod replay;
#[cfg(test)]
mod tests;

use super::surface_preparation::PreparationPolicy;
use kurbo::{Rect, Vec2};
use std::cell::RefCell;
use std::collections::BTreeMap;

type Id = u32;
type Domain = Option<Id>;
type Resource = u8;

/// Spatial demand for one observed viewport and its nested scroll hosts.
/// Surface residency and publication still validate their individual leases.
pub struct ViewportDemand {
    scene: Scene,
    plans: RefCell<BTreeMap<Domain, regions::Plan>>,
}

impl ViewportDemand {
    fn new(scene: Scene) -> Self {
        Self {
            scene,
            plans: Default::default(),
        }
    }

    /// Returns the spatial tier and distance of a rectangle in viewport coordinates.
    pub fn priority(&self, host: Option<u32>, projected: [f64; 4]) -> (u8, u64) {
        let mut plans = self.plans.borrow_mut();
        let plan = plans
            .entry(host)
            .or_insert_with(|| regions::Plan::build(&self.scene, host));
        let bounds = rect(projected) + plan.bias;
        if !bounds.is_finite() || !area(bounds) {
            return (Tier::Remote as u8, u64::MAX);
        }
        (
            plan.tier(bounds) as u8,
            px_key(distance(plan.origin, bounds)),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Host {
    id: Id,
    parent: Domain,
    bounds: Rect,
    clip: Option<Rect>,
    viewport: [f64; 2],
    offset: [f64; 2],
    content: [f64; 2],
    origin: [f64; 2],
    basis: [f64; 4],
    travel: [f64; 2],
}
impl Host {
    fn project(&self, local: [f64; 2]) -> Vec2 {
        let [a, b, c, d] = self.basis;
        Vec2::new(a * local[0] + c * local[1], b * local[0] + d * local[1])
    }
    fn clamp(&self, axis: usize, step: f64) -> f64 {
        let min = self.origin[axis];
        let max = min + (self.content[axis] - self.viewport[axis]).max(0.);
        let at = self.offset[axis].clamp(min, max);
        (at + step).clamp(min, max) - at
    }
    fn bias(&self) -> Vec2 {
        self.project(self.offset) - Vec2::new(self.bounds.x0, self.bounds.y0)
    }
}
#[derive(Clone, Debug, PartialEq)]
struct Scene {
    viewport: Rect,
    active: Domain,
    direction: [i8; 2],
    hosts: BTreeMap<Id, Host>,
    safety: f64,
    reverse: f64,
}
impl Scene {
    fn new(viewport: Rect) -> Self {
        let policy = PreparationPolicy::default();
        Self {
            viewport,
            active: None,
            direction: [0, 0],
            hosts: BTreeMap::new(),
            safety: policy.safety_viewports,
            reverse: policy.reversal_viewports,
        }
    }
    fn bias(&self, domain: Domain) -> Vec2 {
        domain
            .and_then(|id| self.hosts.get(&id))
            .map_or(Vec2::ZERO, Host::bias)
    }
    fn chain(&self, mut domain: Domain) -> Option<Vec<&Host>> {
        let mut chain = Vec::new();
        while let Some(id) = domain {
            let host = self.hosts.get(&id)?;
            if chain.len() >= self.hosts.len() {
                return None;
            }
            domain = host.parent;
            chain.push(host);
        }
        chain.reverse();
        Some(chain)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
enum Tier {
    Visible,
    Leading,
    OtherWarm,
    NestedVisible,
    NestedWarm,
    Forward,
    Reverse,
    Remote,
}
impl Tier {
    fn near(self) -> bool {
        self <= Self::NestedWarm
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Runnable,
    #[cfg(test)]
    Waiting(Wake),
    #[cfg(test)]
    Submitted(u64),
    #[cfg(test)]
    Ready,
    #[cfg(test)]
    Retired,
    Failed(Failure),
}
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wake {
    Measurement(u64),
    Settlement(u64),
    Fence(u64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Failure {
    MissingOwner,
    #[cfg(test)]
    MissingProducer,
}
#[derive(Clone, Debug, PartialEq)]
struct Job {
    age: u32,
    resource: Resource,
    lifetime: u64,
    content: u64,
    owners: Vec<Id>,
    status: Status,
}
#[cfg(test)]
impl Job {
    fn wake(&mut self, lifetime: u64, content: u64, token: Wake) -> bool {
        if (self.lifetime, self.content) != (lifetime, content) {
            return false;
        }
        self.status = match self.status {
            Status::Waiting(t) if t == token => Status::Runnable,
            Status::Submitted(t) if Wake::Fence(t) == token => Status::Ready,
            _ => return false,
        };
        true
    }
}
#[derive(Clone, Debug, PartialEq)]
struct Owner {
    domain: Domain,
    bounds: Option<Rect>,
    epoch: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
struct Choice {
    tier: Tier,
    distance: u64,
    age: u32,
    id: Id,
    host: Domain,
}
pub(super) fn rect(a: [f64; 4]) -> Rect {
    Rect::new(a[0], a[1], a[2], a[3])
}
fn area(r: Rect) -> bool {
    r.x1 > r.x0 + 1e-7 && r.y1 > r.y0 + 1e-7
}
fn distance(a: Rect, b: Rect) -> f64 {
    (a.x0 - b.x1)
        .max(b.x0 - a.x1)
        .max(a.y0 - b.y1)
        .max(b.y0 - a.y1)
        .max(0.)
}
fn px_key(d: f64) -> u64 {
    (d * 64.).round().max(0.) as u64
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Service {
    // A negative balance favors the reverse lane. The caller charges actual
    // bounded microseconds to the lane that consumed the resource.
    weights: [u32; 2],
    max_unit: u32,
    balance: i64,
    serial: u64,
    last: BTreeMap<Domain, u64>,
    direction: Option<(Domain, [i8; 2])>,
}
impl Default for Service {
    fn default() -> Self {
        Self::new([2, 1], 2000)
    }
}
impl Service {
    fn new(weights: [u32; 2], max_unit: u32) -> Self {
        assert!(weights.iter().all(|w| *w > 0) && max_unit > 0);
        Self {
            weights,
            max_unit,
            balance: 0,
            serial: 0,
            last: Default::default(),
            direction: None,
        }
    }
    fn observe(&mut self, scene: &Scene) {
        let next = Some((scene.active, scene.direction));
        if self.direction != next {
            self.balance = 0;
            self.direction = next;
        }
    }
    fn far_order(&self) -> [Tier; 2] {
        if self.balance >= 0 {
            [Tier::Forward, Tier::Reverse]
        } else {
            [Tier::Reverse, Tier::Forward]
        }
    }
    fn charge(&mut self, tier: Tier, host: Domain, micros: u32) {
        self.serial += 1;
        self.last.insert(host, self.serial);
        let n = i64::from(micros.max(1));
        let [ahead, behind] = self.weights.map(i64::from);
        let cap = i64::from(self.max_unit.max(micros));
        self.balance = (self.balance
            + match tier {
                Tier::Forward => -behind * n,
                Tier::Reverse => ahead * n,
                _ => 0,
            })
        .clamp(-behind * cap, ahead * cap);
    }
}

type Order = (u8, u64, u64, u32, Id, Domain);
impl Service {
    fn order(&self, c: Choice) -> Order {
        let (rank, last) = match c.tier {
            Tier::Forward | Tier::Reverse => (if c.tier == self.far_order()[0] { 5 } else { 6 }, 0),
            Tier::Remote => (7, *self.last.get(&c.host).unwrap_or(&0)),
            t => (t as u8, 0),
        };
        (rank, last, c.distance, c.age, c.id, c.host)
    }
}

/// Compare the captured eligible frontier with the read-only proposed selector.
#[cfg(any(test, feature = "scroll-selection-shadow"))]
pub fn replay_capture(input: &str) -> Result<serde_json::Value, String> {
    replay::analyze(input)
}
