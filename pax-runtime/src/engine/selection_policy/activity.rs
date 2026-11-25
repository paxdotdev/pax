use super::*;
use crate::engine::surface_scheduling::ScrollActivityTracker;

#[derive(Clone, Copy)]
pub(super) struct Gesture {
    pub host: Domain,
    pub delta: [f64; 2],
}
pub(super) struct Activity {
    pub tail_ms: f64,
    tracker: ScrollActivityTracker,
    prior: BTreeMap<Id, ([f64; 2], [f64; 2])>,
    owned: BTreeMap<Id, f64>,
    pub active: Domain,
}
impl Default for Activity {
    fn default() -> Self {
        Self::with_tail(250.)
    }
}
impl Activity {
    pub fn with_tail(tail_ms: f64) -> Self {
        Self {
            tail_ms,
            tracker: ScrollActivityTracker::new(tail_ms),
            prior: Default::default(),
            owned: Default::default(),
            active: None,
        }
    }
    pub fn update(
        &mut self,
        scene: &Scene,
        samples: &BTreeMap<Id, [f64; 2]>,
        gestures: &[Gesture],
        at: f64,
    ) {
        self.prior.retain(|id, _| scene.hosts.contains_key(id));
        self.owned.retain(|id, _| scene.hosts.contains_key(id));
        for g in gestures {
            let axis = usize::from(g.delta[1].abs() >= g.delta[0].abs());
            if g.delta[axis] == 0. {
                continue;
            }
            let sign = g.delta[axis].signum();
            let moved = |id: &Id| {
                samples
                    .get(id)
                    .zip(self.prior.get(id))
                    .is_some_and(|(s, (p, _))| (s[axis] - p[axis]) * sign > 0.1)
            };
            let mut cursor = g.host;
            let mut consumed = None;
            for _ in 0..scene.hosts.len() {
                let Some(id) = cursor else { break };
                let Some(h) = scene.hosts.get(&id) else { break };
                if moved(&id) || h.clamp(axis, sign).abs() > 0.1 {
                    consumed = Some(id);
                    break;
                }
                cursor = h.parent;
            }
            if consumed.is_none() {
                let mut candidates = samples.keys().filter(|id| moved(id));
                let first = candidates.next().copied();
                if candidates.next().is_none() {
                    consumed = first;
                }
            }
            if let Some(id) = consumed {
                self.active = Some(id);
                self.owned.insert(id, at);
            }
        }
        for (&id, h) in &scene.hosts {
            let pos = *samples.get(&id).unwrap_or(&h.offset);
            let geometry = format!("{:?}:{:?}:{:?}", h.viewport, h.origin, h.basis);
            let jump = self.prior.get(&id).is_some_and(|(p, _)| {
                (0..2).any(|i| (pos[i] - p[i]).abs() > h.viewport[i].max(1.))
            });
            if jump {
                self.tracker.observe(id, pos, at, "discontinuous offset");
            }
            self.tracker.observe(id, pos, at, &geometry);
            if self.owned.get(&id).is_some_and(|t| at - *t <= self.tail_ms)
                && self
                    .prior
                    .get(&id)
                    .is_some_and(|(p, _)| (0..2).any(|i| (pos[i] - p[i]).abs() > 0.1))
            {
                self.owned.insert(id, at);
            }
            self.prior.insert(id, (pos, h.viewport));
        }
        if self.active.is_none_or(|id| !scene.hosts.contains_key(&id)) {
            self.active = scene
                .hosts
                .values()
                .filter(|h| h.parent.is_none() && (0..2).any(|i| h.content[i] > h.viewport[i]))
                .max_by(|a, b| a.bounds.area().total_cmp(&b.bounds.area()))
                .map(|h| h.id);
        }
    }
    pub fn project(&self, mut scene: Scene, at: f64, horizon: f64) -> Scene {
        scene.active = self.active;
        scene.direction = [0, 0];
        for h in scene.hosts.values_mut() {
            h.travel = [0., 0.];
            let a = self.tracker.get(h.id, at);
            if self.owned.get(&h.id).is_none_or(|t| at - *t > self.tail_ms) {
                continue;
            }
            if Some(h.id) == self.active {
                scene.direction = a.direction.map(|x| x as i8);
            }
            h.travel = [0, 1].map(|i| {
                a.direction[i]
                    * (a.speed[i] * horizon)
                        .min(PreparationPolicy::default().max_viewports * h.viewport[i])
            });
        }
        scene
    }
}
