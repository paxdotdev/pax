//! The admitted unit is still an ordinary detached child attachment. The
//! selector owns weak identities and observations, never a displayed scene.
use super::{
    activity::{Activity, Gesture},
    indexed::{Indexed, Next},
    *,
};
use crate::engine::startup::{
    selection_capture::{footprint, Footprint},
    PendingMount,
};
use crate::engine::surface_preparation::PreparationHost;
use crate::viewport_proximity::{admission_cache::Watched, RankStats};
use crate::{ExpandedNode, RuntimeContext};
use pax_runtime_api::properties::DependencyEventQueue;
use std::collections::{BTreeSet, VecDeque};
use std::rc::{Rc, Weak};
use web_time::Instant;

#[derive(Default, Clone, serde::Serialize)]
pub(crate) struct Counts {
    registered: u32,
    selected: u32,
    discarded: u32,
    geometry_reads: u32,
    maintenance: u32,
    cycles: u32,
    pub grant_count: u32,
    pub grants_with_mount: u32,
    pub peak_frontier: usize,
    pub max_unit_ms: f64,
    pub max_selection_ms: f64,
    pub mounted: u32,
    pub slices_all_maintenance: u32,
    checked: u32,
    disagreements: u32,
    first_disagreement: Option<(u32, Choice, Choice, usize)>,
}
struct Anchor {
    node: Weak<ExpandedNode>,
    users: usize,
    observation: Option<Watched<Footprint>>,
}
struct Ticket {
    work: PendingMount,
    owner: Id,
}
pub(in crate::engine) enum Unit {
    Maintenance,
    Mount(PendingMount),
    Empty,
}
pub(in crate::engine) struct Mounts {
    index: Indexed,
    meter: Service,
    base: Scene,
    demand: Rc<ViewportDemand>,
    activity: Activity,
    gestures: Vec<Gesture>,
    epoch_clock: Instant,
    base_time: f64,
    horizon: f64,
    new_scene: bool,
    jobs: BTreeMap<Id, Ticket>,
    anchors: BTreeMap<Id, Anchor>,
    ready_geometry: VecDeque<Id>,
    dirty: BTreeSet<Id>,
    overflow: BTreeSet<Id>,
    signals: DependencyEventQueue,
    structure: u64,
    cycle: bool,
    watch_stats: RankStats,
    chosen: Option<Choice>,
    check_limit: u32,
    check_stride: u32,
    pub counts: Counts,
}
impl Mounts {
    pub(in crate::engine) fn new(at: f64) -> Self {
        let base = Scene::new(Rect::ZERO);
        Self {
            index: Indexed::new(base.clone()),
            demand: Rc::new(ViewportDemand::new(base.clone())),
            base,
            activity: Default::default(),
            gestures: Vec::new(),
            epoch_clock: Instant::now(),
            base_time: at,
            horizon: 250.,
            new_scene: false,
            jobs: Default::default(),
            anchors: Default::default(),
            ready_geometry: Default::default(),
            dirty: Default::default(),
            signals: Default::default(),
            structure: 0,
            cycle: false,
            watch_stats: Default::default(),
            chosen: None,
            check_limit: 0,
            check_stride: 1,
            meter: Default::default(),
            counts: Default::default(),
            overflow: Default::default(),
        }
    }
    pub(in crate::engine) fn len(&self) -> usize {
        self.jobs.len()
    }
    pub(in crate::engine) fn needs_first_observation(&self) -> bool {
        self.base.viewport == Rect::ZERO || !self.gestures.is_empty()
    }
    pub(in crate::engine) fn any_legal(&self) -> bool {
        self.jobs.values().any(|t| t.work.legal_now())
    }
    fn now(&self) -> f64 {
        self.base_time + self.epoch_clock.elapsed().as_secs_f64() * 1000.
    }
    fn mark(&mut self, id: Id) {
        if self.anchors.contains_key(&id) && self.dirty.insert(id) {
            self.ready_geometry.push_back(id);
        }
    }
    fn observe(
        &mut self,
        now: f64,
        viewport: [f64; 4],
        hosts: &BTreeMap<Id, PreparationHost>,
        observed: &[(Id, [f64; 2])],
        horizon: f64,
    ) {
        let mut scene = Scene::new(rect(viewport));
        for (&id, h) in hosts {
            let g = &h.geometry;
            let edges = |r: crate::engine::surface_scheduling::CoverageRect| {
                Rect::new(r.left, r.top, r.right, r.bottom)
            };
            scene.hosts.insert(
                id,
                Host {
                    id,
                    parent: g.parent,
                    bounds: edges(g.bounds),
                    clip: h.intermediate_clip.map(edges),
                    viewport: g.viewport,
                    offset: g.offset,
                    content: h.content,
                    origin: h.content_origin,
                    basis: g.basis,
                    travel: [0.; 2],
                },
            );
        }
        if scene == self.base && self.gestures.is_empty() && self.horizon == horizon {
            return;
        }
        self.activity.update(
            &scene,
            &observed.iter().copied().collect(),
            &self.gestures,
            now,
        );
        self.gestures.clear();
        self.base = scene;
        self.horizon = horizon;
        self.new_scene = true;
    }
    pub(in crate::engine) fn begin_grant(&mut self) {
        self.counts.grant_count += 1;
        // Time alone can expire momentum between two identical host observations.
        if self.new_scene
            || self.index.scene.direction != [0, 0]
            || self
                .index
                .scene
                .hosts
                .values()
                .any(|h| h.travel != [0., 0.])
        {
            let demand = self.demand_at(self.now());
            let next = &demand.scene;
            if &self.index.scene != next {
                let shifted: Vec<_> = self
                    .anchors
                    .iter()
                    .filter_map(|(&id, a)| {
                        let d = a.observation.as_ref()?.value.host;
                        (self.index.scene.bias(d) != next.bias(d)).then_some(id)
                    })
                    .collect();
                self.index.observe(next.clone());
                for id in shifted {
                    self.mark(id);
                }
                self.meter.observe(&self.index.scene);
            }
            self.new_scene = false;
        }
    }
    fn begin_cycle(&mut self, ctx: &RuntimeContext) {
        if !self.cycle {
            self.cycle = true;
            self.counts.cycles += 1;
            for id in self.overflow.iter().copied().collect::<Vec<_>>() {
                self.mark(id);
            }
        }
        // A budget can interrupt index maintenance before a mount is chosen.
        // Observe writes from the intervening foreground tick before resuming.
        let revision = ctx.admission_changes.borrow().revision;
        if self.structure != revision {
            self.structure = revision;
            for id in self.anchors.keys().copied().collect::<Vec<_>>() {
                self.mark(id);
            }
        }
        while let Some(id) = self.signals.pop() {
            self.mark(id as Id);
        }
    }
    fn demand_at(&mut self, at: f64) -> Rc<ViewportDemand> {
        let scene = self.activity.project(self.base.clone(), at, self.horizon);
        if scene != self.demand.scene {
            self.demand = Rc::new(ViewportDemand::new(scene));
        }
        self.demand.clone()
    }
    fn add_anchor(&mut self, a: &Rc<ExpandedNode>) {
        let id = a.id.to_u32();
        if let Some(old) = self.anchors.get_mut(&id) {
            old.users += 1;
            return;
        }
        self.anchors.insert(
            id,
            Anchor {
                node: Rc::downgrade(a),
                users: 1,
                observation: None,
            },
        );
        self.mark(id);
    }
    fn drop_anchor(&mut self, id: Id) {
        let anchor = self.anchors.get_mut(&id).expect("ticket has an anchor");
        anchor.users -= 1;
        if anchor.users == 0 {
            self.anchors.remove(&id);
            self.dirty.remove(&id);
            self.overflow.remove(&id);
            self.index.remove_owner(id);
        }
    }
    fn refresh(&mut self, id: Id, ctx: &RuntimeContext) {
        let Some(a) = self.anchors.get_mut(&id) else {
            return;
        };
        let Some(node) = a.node.upgrade() else {
            self.index.put_owner(
                id,
                Owner {
                    domain: None,
                    bounds: None,
                    epoch: 0,
                },
            );
            return;
        };
        // The same dependency capture backed the independent Web validation.
        let fp = Watched::observe(
            a.observation.as_mut(),
            || footprint(ctx, &node),
            &self.signals,
            u64::from(id),
            &mut self.watch_stats,
        );
        if fp.reusable() {
            self.overflow.remove(&id);
        } else {
            self.overflow.insert(id);
        }
        let f = &fp.value;
        let b = f.clipped_bounds();
        self.index.put_owner(
            id,
            Owner {
                domain: f.host,
                bounds: b.map(|b| b + self.index.scene.bias(f.host)),
                epoch: f.epoch,
            },
        );
        a.observation = Some(fp);
        self.counts.geometry_reads += 1;
    }
    fn input_job(&mut self, id: Id, owner: Id, epoch: u64, age: u32) {
        self.index.put_job(
            id,
            Job {
                age,
                lifetime: epoch,
                content: 0,
                resource: 0,
                owners: vec![owner],
                status: Status::Runnable,
            },
        );
    }
    fn register(&mut self, ctx: &RuntimeContext, work: PendingMount) -> Unit {
        let (Some(parent), Some(child)) = (work.parent.upgrade(), work.child.upgrade()) else {
            return Unit::Mount(work);
        };
        if !work.runnable(&parent, &child) {
            return Unit::Mount(work);
        }
        let (_, a, age) =
            ctx.visible_groups
                .borrow()
                .scheduling_anchor(&parent, &child, work.arrival);
        let owner = a.id.to_u32();
        let id = work.arrival;
        self.add_anchor(&a);
        self.input_job(id, owner, work.child_epoch, age);
        assert!(
            self.jobs.insert(id, Ticket { work, owner }).is_none(),
            "duplicate live mount ticket"
        );
        self.counts.registered += 1;
        self.counts.peak_frontier = self.counts.peak_frontier.max(self.jobs.len());
        Unit::Maintenance
    }
    pub(in crate::engine) fn step(
        &mut self,
        ctx: &RuntimeContext,
        incoming: Option<PendingMount>,
    ) -> Unit {
        if let Some(work) = incoming {
            return self.register(ctx, work);
        }
        if self.jobs.is_empty() {
            return Unit::Empty;
        }
        self.begin_cycle(ctx);
        while let Some(id) = self.ready_geometry.pop_front() {
            if self.dirty.remove(&id) {
                self.refresh(id, ctx);
                return Unit::Maintenance;
            }
        }
        match self.index.next(0, &self.meter) {
            Next::Refresh => {
                self.index.settle(1);
                Unit::Maintenance
            }
            Next::Selected(Some(choice)) => {
                if self.counts.checked < self.check_limit
                    && self.counts.selected % self.check_stride == 0
                {
                    self.check_choice(ctx, choice);
                }
                let ticket = self
                    .jobs
                    .remove(&choice.id)
                    .expect("indexed ticket remains live");
                self.index.remove_job(choice.id);
                self.drop_anchor(ticket.owner);
                self.cycle = false;
                self.chosen = Some(choice);
                self.counts.selected += 1;
                Unit::Mount(ticket.work)
            }
            Next::Selected(None) => {
                panic!("live mount frontier has no reachable runnable ticket")
            }
        }
    }
    // Only a separately requested checked run reads the full live frontier.
    // Reconstruct owner, inherited age and footprint from source identities so
    // a ticket omitted or left stale by the spatial index is still a contender.
    fn check_choice(&mut self, ctx: &RuntimeContext, actual: Choice) {
        if !self.jobs[&actual.id].work.legal_now() {
            return;
        }
        let mut jobs = BTreeMap::new();
        let mut owners = BTreeMap::new();
        let groups = ctx.visible_groups.borrow();
        for (&id, ticket) in &self.jobs {
            let w = &ticket.work;
            let (Some(parent), Some(child)) = (w.parent.upgrade(), w.child.upgrade()) else {
                continue;
            };
            if !w.runnable(&parent, &child) {
                continue;
            }
            let (_, node, age) = groups.scheduling_anchor(&parent, &child, w.arrival);
            let owner = node.id.to_u32();
            owners.entry(owner).or_insert_with(|| {
                let f = footprint(ctx, &node);
                Owner {
                    domain: f.host,
                    bounds: f
                        .clipped_bounds()
                        .map(|r| r + self.index.scene.bias(f.host)),
                    epoch: f.epoch,
                }
            });
            jobs.insert(
                id,
                Job {
                    age,
                    lifetime: w.child_epoch,
                    content: 0,
                    resource: 0,
                    status: Status::Runnable,
                    owners: vec![owner],
                },
            );
        }
        let choices = super::reference::Reference::new(&self.index.scene).enumerate(
            &jobs,
            &owners,
            0,
            &self.meter,
        );
        let expected = choices
            .values()
            .min_by_key(|&&c| self.meter.order(c))
            .copied()
            .expect("checked a live mount");
        self.counts.checked += 1;
        if expected != actual {
            self.counts.disagreements += 1;
            self.counts.first_disagreement.get_or_insert((
                self.counts.selected,
                actual,
                expected,
                jobs.len(),
            ));
        }
    }
    pub(in crate::engine) fn finish_mount(&mut self, child: Id, mounted: bool, micros: u32) {
        if let Some(c) = self.chosen.take() {
            if mounted {
                self.meter.charge(c.tier, c.host, micros);
            }
        }
        if !mounted {
            self.counts.discarded += 1;
            return;
        }
        self.counts.mounted += 1;
        // Attachment changes the lifetime and can replace a parent's discovery
        // rectangle. These cells do not emit property invalidations.
        if let Some(anchor) = self.anchors.get_mut(&child) {
            if let Some(observation) = &mut anchor.observation {
                observation.invalidate();
            }
            self.mark(child);
        }
    }
    pub(in crate::engine) fn counted(&mut self, ms: f64, maintenance: bool) {
        self.counts.max_unit_ms = self.counts.max_unit_ms.max(ms);
        if maintenance {
            self.counts.maintenance += 1;
            self.counts.max_selection_ms = self.counts.max_selection_ms.max(ms);
        }
    }
}
impl crate::PaxEngine {
    pub fn check_mount_selection(&self, limit: u32, stride: u32) {
        if let Some(live) = self
            .runtime_context
            .startup
            .borrow_mut()
            .live_mounts
            .as_mut()
        {
            live.check_limit = limit;
            live.check_stride = stride.max(1);
        }
    }
    pub fn enable_indexed_mounts(&self, at: f64) {
        self.runtime_context.startup.borrow_mut().live_mounts = Some(Mounts::new(at));
    }
    pub fn mount_scroll_intent(&self, host: Option<u32>, dx: f64, dy: f64) {
        let mut state = self.runtime_context.startup.borrow_mut();
        if !state.accepts_mount_input() && !state.share_viewport_demand {
            return;
        }
        if let Some(l) = state.live_mounts.as_mut() {
            l.gestures.push(Gesture {
                host,
                delta: [dx, dy],
            });
        }
    }
    pub fn observe_mount_hosts(
        &self,
        now: f64,
        viewport: [f64; 4],
        hosts: &BTreeMap<u32, PreparationHost>,
        observed: &[(u32, [f64; 2])],
        horizon: f64,
    ) -> Option<Rc<ViewportDemand>> {
        let mut state = self.runtime_context.startup.borrow_mut();
        let shared = state.share_viewport_demand;
        let live = state.live_mounts.as_mut()?;
        live.observe(now, viewport, hosts, observed, horizon);
        shared.then(|| live.demand_at(now))
    }
    pub fn share_viewport_demand(&self) {
        self.runtime_context
            .startup
            .borrow_mut()
            .share_viewport_demand = true;
    }
    pub fn ordered_visible_groups(&self) -> Vec<(u32, u64, Vec<u32>)> {
        let ctx = &self.runtime_context;
        let mut ready = ctx.ready_visible_groups();
        if ready.len() > 1 {
            let state = ctx.startup.borrow();
            let demand = state
                .live_mounts
                .as_ref()
                .filter(|_| state.share_viewport_demand)
                .map(|live| live.demand.clone());
            drop(state);
            if let Some(demand) = demand {
                ready.sort_by_cached_key(|(id, _, _)| {
                    ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(*id))
                        .and_then(|node| {
                            let f = footprint(ctx, &node);
                            let bounds = f.clipped_bounds()?;
                            Some(
                                demand
                                    .priority(f.host, [bounds.x0, bounds.y0, bounds.x1, bounds.y1]),
                            )
                        })
                        .unwrap_or((Tier::Remote as u8, u64::MAX))
                });
            }
        }
        ready
    }
    pub fn live_mount_counts(&self) -> impl serde::Serialize {
        self.runtime_context
            .startup
            .borrow()
            .live_mounts
            .as_ref()
            .map(|v| v.counts.clone())
    }
}
