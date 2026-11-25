use super::{regions::Plan, *};
use crate::scene_geometry::SpatialIndex;
use std::collections::{BTreeSet, HashSet};

#[derive(Default)]
struct DomainIndex {
    spatial: SpatialIndex,
    members: BTreeSet<Id>,
}
#[derive(Default, Debug, serde::Serialize)]
pub(super) struct Counters {
    pub classifications: u64,
    pub region_candidates: u64,
    pub cold_candidates: u64,
    pub plans: u64,
    pub lookups: u64,
}
#[derive(Debug, PartialEq)]
pub(super) enum Next {
    Refresh,
    Selected(Option<Choice>),
}

pub(super) struct Indexed {
    pub scene: Scene,
    plans: BTreeMap<Domain, Plan>,
    domains: BTreeMap<Domain, DomainIndex>,
    ancestors: BTreeMap<Id, BTreeSet<Domain>>,
    paths: BTreeMap<Domain, Vec<Id>>,
    owners: BTreeMap<Id, Owner>,
    jobs: BTreeMap<Id, Job>,
    consumers: BTreeMap<Id, BTreeSet<Id>>,
    dirty: BTreeSet<(Tier, Id)>,
    dirty_band: BTreeMap<Id, Tier>,
    near_owner: BTreeMap<Id, (Tier, u64, Domain)>,
    tickets: BTreeMap<Id, Choice>,
    queues: BTreeMap<Resource, BTreeSet<Choice>>,
    last: BTreeMap<Resource, (Service, Option<Choice>)>,
    pub stats: Counters,
}
impl Indexed {
    pub fn new(scene: Scene) -> Self {
        Self {
            scene,
            plans: Default::default(),
            domains: Default::default(),
            owners: Default::default(),
            ancestors: Default::default(),
            paths: Default::default(),
            jobs: Default::default(),
            consumers: Default::default(),
            dirty: Default::default(),
            dirty_band: Default::default(),
            near_owner: Default::default(),
            tickets: Default::default(),
            queues: Default::default(),
            last: Default::default(),
            stats: Default::default(),
        }
    }
    fn track_path(&mut self, domain: Domain) {
        let mut path = Vec::new();
        let mut at = domain;
        while let Some(id) = at {
            if path.contains(&id) {
                break;
            }
            path.push(id);
            at = self.scene.hosts.get(&id).and_then(|h| h.parent);
        }
        if self.paths.get(&domain) == Some(&path) {
            return;
        }
        self.forget_path(domain);
        for &id in &path {
            self.ancestors.entry(id).or_default().insert(domain);
        }
        self.paths.insert(domain, path);
    }
    fn forget_path(&mut self, domain: Domain) {
        if let Some(path) = self.paths.remove(&domain) {
            for id in path {
                if let Some(set) = self.ancestors.get_mut(&id) {
                    set.remove(&domain);
                    if set.is_empty() {
                        self.ancestors.remove(&id);
                    }
                }
            }
        }
    }
    fn drop_member(&mut self, domain: Domain, id: Id) {
        let d = self.domains.get_mut(&domain).unwrap();
        d.members.remove(&id);
        d.spatial.remove(id);
        if d.members.is_empty() {
            self.domains.remove(&domain);
            self.plans.remove(&domain);
            self.forget_path(domain);
        }
    }
    fn ensure(&mut self, domain: Domain) {
        if self.plans.contains_key(&domain) {
            return;
        }
        self.stats.plans += 1;
        self.plans.insert(domain, Plan::build(&self.scene, domain));
        self.domains.entry(domain).or_default();
        self.track_path(domain);
    }
    fn invalidate(&mut self, id: Id, possible: Tier) {
        // A cached higher-priority ticket may now have become stale. Either
        // its old rank or its new bound blocks lower service until repaired.
        let bound = self
            .near_owner
            .get(&id)
            .map_or(possible, |old| possible.min(old.0));
        if let Some(old) = self.dirty_band.insert(id, bound) {
            self.dirty.remove(&(old, id));
        }
        self.dirty.insert((bound, id));
    }
    pub fn observe(&mut self, next: Scene) {
        if self.scene == next {
            return;
        }
        let mut hosts = BTreeSet::new();
        for (&id, h) in &next.hosts {
            if self.scene.hosts.get(&id) != Some(h) {
                hosts.insert(id);
            }
        }
        hosts.extend(
            self.scene
                .hosts
                .keys()
                .copied()
                .filter(|id| !next.hosts.contains_key(id)),
        );
        if (self.scene.active, self.scene.direction) != (next.active, next.direction) {
            hosts.extend(self.scene.active);
            hosts.extend(next.active);
        }
        let mut affected: BTreeSet<Domain> = hosts
            .iter()
            .flat_map(|id| self.ancestors.get(id).into_iter().flatten().copied())
            .collect();
        if (self.scene.viewport, self.scene.safety, self.scene.reverse)
            != (next.viewport, next.safety, next.reverse)
        {
            affected.extend(self.domains.keys().copied());
        }
        for &domain in &affected {
            let index = &self.domains[&domain];
            let plan = Plan::build(&next, domain);
            self.stats.plans += 1;
            let prior = &self.plans[&domain];
            let mut ids = HashSet::new();
            for &(_, r) in prior.near.iter().chain(&plan.near) {
                index.spatial.candidates(r, &mut ids);
            }
            self.stats.region_candidates += ids.len() as u64;
            let mut changed = Vec::new();
            for id in ids {
                if self.owners[&id].bounds.is_some_and(|b| {
                    prior
                        .near
                        .iter()
                        .chain(&plan.near)
                        .any(|(_, r)| area(r.intersect(b)))
                }) {
                    changed.push((id, plan.possible(self.owners[&id].bounds)));
                }
            }
            self.plans.insert(domain, plan);
            for (id, bound) in changed {
                self.invalidate(id, bound);
            }
        }
        self.scene = next;
        for domain in affected {
            self.track_path(domain);
        }
        self.last.clear();
    }
    pub fn put_owner(&mut self, id: Id, owner: Owner) {
        if self.owners.get(&id) == Some(&owner) {
            return;
        }
        if let Some(old) = self.owners.get(&id) {
            if old.domain != owner.domain {
                self.drop_member(old.domain, id);
            }
        }
        self.ensure(owner.domain);
        let index = self.domains.get_mut(&owner.domain).unwrap();
        index.members.insert(id);
        if let Some(b) = owner.bounds.filter(|b| area(*b)) {
            index.spatial.insert(id, Some(b));
        } else {
            index.spatial.remove(id);
        }
        let band = self.plans[&owner.domain].possible(owner.bounds);
        self.owners.insert(id, owner);
        self.invalidate(id, band);
        self.last.clear();
    }
    pub fn remove_owner(&mut self, id: Id) {
        if let Some(o) = self.owners.remove(&id) {
            self.drop_member(o.domain, id);
            self.invalidate(id, Tier::Forward);
            self.last.clear();
        }
    }
    pub fn put_job(&mut self, id: Id, mut job: Job) {
        job.owners.sort_unstable();
        job.owners.dedup();
        if job.owners.is_empty() && job.status == Status::Runnable {
            job.status = Status::Failed(Failure::MissingOwner);
        }
        if self.jobs.get(&id) == Some(&job) {
            return;
        }
        self.remove_job(id);
        for &o in &job.owners {
            self.consumers.entry(o).or_default().insert(id);
        }
        self.jobs.insert(id, job);
        self.rekey(id);
        self.last.clear();
    }
    pub fn remove_job(&mut self, id: Id) {
        if let Some(old) = self.jobs.remove(&id) {
            if let Some(c) = self.tickets.remove(&id) {
                self.queues.get_mut(&old.resource).unwrap().remove(&c);
            }
            for o in old.owners {
                if let Some(ids) = self.consumers.get_mut(&o) {
                    ids.remove(&id);
                    if ids.is_empty() {
                        self.consumers.remove(&o);
                    }
                }
            }
            self.last.clear();
        }
    }
    #[cfg(test)]
    pub fn signal(&mut self, id: Id, lifetime: u64, revision: u64, key: Wake) -> bool {
        let Some(j) = self.jobs.get_mut(&id) else {
            return false;
        };
        if !j.wake(lifetime, revision, key) {
            return false;
        }
        self.rekey(id);
        self.last.clear();
        true
    }
    #[cfg(test)]
    pub fn outcome(&self, id: Id) -> Option<Status> {
        self.jobs.get(&id).map(|j| j.status)
    }
    fn rekey(&mut self, id: Id) {
        let Some(j) = self.jobs.get(&id) else { return };
        let q = self.queues.entry(j.resource).or_default();
        if let Some(old) = self.tickets.remove(&id) {
            q.remove(&old);
        }
        if j.status != Status::Runnable {
            return;
        }
        if let Some(c) = j
            .owners
            .iter()
            .filter_map(|o| self.near_owner.get(o))
            .map(|&(tier, distance, host)| Choice {
                tier,
                distance,
                host,
                age: j.age,
                id,
            })
            .min()
        {
            q.insert(c);
            self.tickets.insert(id, c);
        }
    }
    pub fn settle(&mut self, limit: usize) {
        let mut changed = BTreeSet::new();
        for _ in 0..limit {
            let Some((_, id)) = self.dirty.pop_first() else {
                break;
            };
            self.dirty_band.remove(&id);
            self.stats.classifications += 1;
            let rank = self.owners.get(&id).and_then(|o| {
                let b = o.bounds.filter(|b| area(*b))?;
                let p = &self.plans[&o.domain];
                let tier = p.tier(b);
                tier.near()
                    .then(|| (tier, px_key(distance(p.origin, b)), o.domain))
            });
            if self.near_owner.get(&id).copied() == rank {
                continue;
            }
            if let Some(rank) = rank {
                self.near_owner.insert(id, rank);
            } else {
                self.near_owner.remove(&id);
            }
            changed.extend(self.consumers.get(&id).into_iter().flatten().copied());
        }
        for id in changed {
            self.rekey(id);
        }
    }
    // Distant lanes pay only when near queues are empty. Search expanding
    // rectangles in normalized host space and stop once the distance bound
    // proves that all candidates which could tie the current winner were seen.
    fn cold(&mut self, resource: Resource, s: &Service) -> Option<Choice> {
        for tier in s.far_order().into_iter().chain([Tier::Remote]) {
            let mut best: Option<Choice> = None;
            let mut domains: Vec<_> = self.domains.keys().copied().collect();
            if tier == Tier::Remote {
                domains.sort_by_key(|d| (s.last.get(d).copied().unwrap_or(0), *d));
            }
            for domain in domains {
                if tier == Tier::Remote
                    && best.is_some_and(|c| {
                        s.last.get(&domain).copied().unwrap_or(0)
                            > s.last.get(&c.host).copied().unwrap_or(0)
                    })
                {
                    break;
                }
                let p = &self.plans[&domain];
                let d = &self.domains[&domain];
                let extent = match tier {
                    Tier::Forward => p.far[0],
                    Tier::Reverse => p.far[1],
                    _ => None,
                };
                if tier != Tier::Remote && extent.is_none() {
                    continue;
                }
                let mut visited = HashSet::new();
                let mut radius = 64.0_f64;
                loop {
                    let full = radius > 1e8;
                    let mut hits = HashSet::new();
                    if full {
                        hits.extend(d.members.iter().copied());
                    } else {
                        let window = p.origin.inflate(radius, radius);
                        let search = extent.map_or(window, |r| r.intersect(window));
                        if area(search) {
                            d.spatial.candidates(search, &mut hits);
                        }
                    }
                    for id in hits {
                        if !visited.insert(id) {
                            continue;
                        }
                        let Some(jobs) = self.consumers.get(&id) else {
                            continue;
                        };
                        let o = &self.owners[&id];
                        let b = o.bounds.filter(|r| area(*r));
                        self.stats.cold_candidates += 1;
                        if b.map_or(Tier::Remote, |r| p.tier(r)) != tier {
                            continue;
                        }
                        for &job_id in jobs {
                            let j = &self.jobs[&job_id];
                            if j.resource != resource || j.status != Status::Runnable {
                                continue;
                            }
                            let c = Choice {
                                tier,
                                distance: b.map_or(u64::MAX, |r| px_key(distance(p.origin, r))),
                                age: j.age,
                                id: job_id,
                                host: domain,
                            };
                            if best.is_none_or(|old| s.order(c) < s.order(old)) {
                                best = Some(c);
                            }
                        }
                    }
                    if full
                        || best.is_some_and(|c| {
                            s.order(c).1 == *s.last.get(&domain).unwrap_or(&0)
                                || tier != Tier::Remote
                        }) && best.is_some_and(|c| c.distance < px_key(radius))
                        || extent.is_some_and(|r| p.origin.inflate(radius, radius).contains_rect(r))
                    {
                        break;
                    }
                    radius *= 4.;
                }
            }
            if best.is_some() {
                return best;
            }
        }
        None
    }
    pub fn next(&mut self, resource: Resource, s: &Service) -> Next {
        let head = self.queues.get(&resource).and_then(|q| q.first()).copied();
        if let Some(&(bound, _)) = self.dirty.first() {
            if head.is_none_or(|c| c.tier >= bound) {
                return Next::Refresh;
            }
        }
        self.stats.lookups += 1;
        if let Some(c) = head {
            return Next::Selected(Some(c));
        }
        if let Some((prior, c)) = self.last.get(&resource) {
            if prior == s {
                return Next::Selected(*c);
            }
        }
        let c = self.cold(resource, s);
        self.last.insert(resource, (s.clone(), c));
        Next::Selected(c)
    }
}
