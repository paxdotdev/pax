//! Experimental, host-serviced initial mounting. A continuation owns identities
//! and phase cursors only; no property evaluation or borrow crosses a host yield.
use super::occlusion;
use super::selection_policy::live::{Mounts, Unit as SelectionUnit};
pub(crate) mod admission;
pub mod plan;
mod profile;
pub mod selection_capture;
pub(crate) mod visible_groups;
use crate::{ExpandedNode, ExpandedNodeIdentifier, PaxEngine, RuntimeContext};
use pax_runtime_api::properties;
pub use plan::{StartupAxes, StartupDemand, StartupPlan};
use plan::{StartupRequirements, StartupResolver};
use profile::{StartupPhaseGuard, StartupPhases, StartupProfile};
pub use profile::{StartupPhaseProfile, StartupProfileReport, StartupUnitProfile};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    rc::{Rc, Weak},
};

// Soft admission threshold: one mount or feedback event can exceed it. Drain a
// batch before admitting more background work so late viewport feedback does not
// inherit the entire page's mount backlog. This is not a per-unit time bound.
const BACKGROUND_EFFECT_LIMIT: usize = 2048;

#[derive(Default)]
pub(crate) struct Startup {
    profile: Option<StartupProfile>,
    // Kept separate while service owns the report, so nested callbacks can record
    // phases without holding a Startup borrow across application code.
    profile_phases: Option<Rc<std::cell::RefCell<StartupPhases>>>,
    active: bool,
    ready: bool,
    viewport_published: bool,
    background: bool,
    viewport_order: bool,
    viewport_changed: bool,
    classifying_frontier: bool,
    warm_turn: bool,
    admission_counts: [u32; 3],
    // Structural effects can realize an admitted parent's children in a later
    // foreground tick. Keep those initial descendants under the same budget.
    background_parents: HashSet<(ExpandedNodeIdentifier, u64)>,
    awaiting_background_settlement: bool,
    settling_viewport: bool,
    measurements_ready: bool,
    prioritize_owners: bool,
    pub(super) live_mounts: Option<Mounts>,
    pub(super) share_viewport_demand: bool,
    enqueue_order: u32,
    resume_mode: u8,
    measure_selection: bool,
    admission_view: Option<crate::viewport_proximity::AdmissionView>,
    rank_stats: crate::viewport_proximity::RankStats,
    spatial: admission::AdmissionQueue,
    incoming: Vec<PendingMount>,
    stack: VecDeque<PendingMount>,
    deferred: VecDeque<PendingMount>,
    plans: HashMap<(ExpandedNodeIdentifier, u64), (StartupDemand, Rc<StartupRequirements>)>,
    demands: HashMap<(ExpandedNodeIdentifier, u64), StartupDemand>,
    measured: HashMap<(ExpandedNodeIdentifier, u64), StartupDemand>,
    resolvers: HashMap<(ExpandedNodeIdentifier, u64), StartupResolver>,
    measurement_axes: HashMap<(ExpandedNodeIdentifier, u64), StartupAxes>,
    queued: HashSet<(ExpandedNodeIdentifier, u64, ExpandedNodeIdentifier, u64)>,
    phase: Phase,
    handlers: Option<VecDeque<ExpandedNodeIdentifier>>,
    ticked: HashSet<(ExpandedNodeIdentifier, u64)>,
    pre_rendered: HashSet<(ExpandedNodeIdentifier, u64)>,
    clock_advanced: bool,
    draining_background_effects: bool,
}

impl Startup {
    pub(super) fn accepts_mount_input(&self) -> bool {
        self.active || self.background
    }
}
fn startup_key(node: &ExpandedNode) -> (ExpandedNodeIdentifier, u64) {
    (
        node.id,
        node.mount_epoch
            .get()
            .wrapping_add(u64::from(node.attached.get() == 0)),
    )
}

#[derive(Clone)]
pub(super) struct PendingMount {
    pub(super) arrival: u32,
    pub(super) parent: Weak<ExpandedNode>,
    pub(super) child: Weak<ExpandedNode>,
    pub(super) parent_epoch: u64,
    pub(super) child_epoch: u64,
    pub(super) key: (ExpandedNodeIdentifier, u64, ExpandedNodeIdentifier, u64),
}

impl PendingMount {
    pub(super) fn legal_now(&self) -> bool {
        self.parent
            .upgrade()
            .zip(self.child.upgrade())
            .is_some_and(|(p, c)| self.runnable(&p, &c))
    }
    pub(super) fn runnable(&self, parent: &Rc<ExpandedNode>, child: &Rc<ExpandedNode>) -> bool {
        child.attached.get() == 0
            && parent.mount_epoch.get() == self.parent_epoch
            && child.mount_epoch.get() == self.child_epoch
            && parent.accepts_pending_child(child)
            && (child
                .render_parent_node()
                .is_some_and(|p| Rc::ptr_eq(&p, parent))
                // Overlays escape render ancestry but remain owned by the
                // attachment that queued their mount. Validate that attachment
                // so a stale continuation cannot mount a removed contribution.
                || (child.instance_node.borrow().is_overlay()
                    && child.authored_attachment.borrow().as_ref().is_some_and(|a| {
                        a.parent.upgrade().is_some_and(|p| Rc::ptr_eq(&p, parent))
                    })))
    }
}

enum MountOutcome {
    Mounted(Rc<ExpandedNode>),
    Deferred,
    Stale,
}

#[derive(Clone, Copy, Default)]
enum Phase {
    #[default]
    InitialSettlement,
    Imperative,
    Async,
    Tick,
    PreRender,
    Clock,
    Geometry,
    Proximity,
    Occlusion,
    FinalGeometry,
    Complete,
}

/// CPU-side startup service counters. A unit may exceed the requested budget:
/// arbitrary Rust callbacks and structural dependency groups are indivisible.
#[derive(Default, pax_message::serde::Serialize)]
#[serde(crate = "pax_message::serde")]
pub struct StartupReport {
    pub pending: bool,
    pub ready_to_present: bool,
    pub deferred_roots: usize,
    pub units: usize,
    /// Mount continuations attempted, including deferred or discarded stale entries.
    pub mounted: usize,
    /// Queued reactive evaluations, including both effects and cutoff propagation.
    pub effects: usize,
    /// Largest queued effect/cutoff count observed during this service call.
    /// Includes entries already settled by synchronous reads but not yet popped.
    pub peak_queued_effects: usize,
    pub elapsed_ms: f64,
    pub longest_unit_ms: f64,
    pub longest_unit: &'static str,
}

impl RuntimeContext {
    pub(super) fn settled_admission(&self) {
        self.startup.borrow_mut().awaiting_background_settlement = false;
        self.visible_groups.borrow_mut().did_settle();
    }

    pub(crate) fn startup_phase(&self, name: &'static str) -> Option<StartupPhaseGuard> {
        let state = self.startup.borrow();
        if !state.active {
            return None;
        }
        state.profile_phases.as_ref().map(|phases| StartupPhaseGuard::new(phases.clone(), name))
    }

    fn startup_demand(&self, node: &ExpandedNode) -> Option<StartupDemand> {
        let demand = if let Some(parent) = node.render_parent_node() {
            let inherited = self.startup_demand(&parent)?;
            let key = startup_key(&parent);
            let cached = self.startup.borrow().plans.get(&key).cloned();
            let requirements = match cached {
                Some((previous, plan)) if previous == inherited => plan,
                _ => {
                    let mut plan = StartupPlan::new(self, &parent, inherited);
                    let resolver = self.startup.borrow().resolvers.get(&key).cloned();
                    if let Some(resolver) = resolver {
                        resolver(&parent, &mut plan);
                    } else {
                        parent
                            .instance_node
                            .borrow()
                            .plan_startup(&parent, &mut plan);
                    }
                    let requirements = Rc::new(plan.finish());
                    self.startup
                        .borrow_mut()
                        .plans
                        .insert(key, (inherited, requirements.clone()));
                    requirements
                }
            };
            requirements.child(node, inherited)?
        } else {
            let (width, height) = self.globals().viewport.get().bounds;
            StartupDemand::visible(kurbo::Rect::new(0.0, 0.0, width, height))
        };
        if !demand.complete && !demand.measure.any() {
            if let Some(region) = demand.region {
                let common = node.get_common_properties();
                let common = common.borrow();
                // Unknown dimensions are candidates, not zero-sized exclusions.
                if common.width.get().is_some() && common.height.get().is_some() {
                    if let Some(bounds) = self.startup_bounds(node) {
                        if bounds.width() > 0.0
                            && bounds.height() > 0.0
                            && (bounds.x1 < region.x0
                                || bounds.y1 < region.y0
                                || bounds.x0 > region.x1
                                || bounds.y0 > region.y1)
                        {
                            return None;
                        }
                    }
                }
            }
        }
        self.startup
            .borrow_mut()
            .demands
            .insert(startup_key(node), demand);
        Some(demand)
    }

    fn startup_bounds(&self, node: &ExpandedNode) -> Option<kurbo::Rect> {
        let tab = node.presentation_transform_and_bounds.get();
        let transform =
            self.presentation_scroll_transform_for_node(node) * kurbo::Affine::from(tab.transform);
        let bounds =
            transform.transform_rect_bbox(kurbo::Rect::new(0.0, 0.0, tab.bounds.0, tab.bounds.1));
        bounds.is_finite().then_some(bounds)
    }

    /// Register axes whose extent is measured from this container's children.
    /// The default planner preserves exact measurement dependencies while allowing
    /// a top-aligned, fixed-width visible prefix to remain partially mounted.
    pub fn register_startup_measurement(&self, node: &ExpandedNode, axes: StartupAxes) {
        let mut state = self.startup.borrow_mut();
        if state.active {
            let key = startup_key(node);
            if state.measurement_axes.insert(key, axes) != Some(axes) {
                state.plans.remove(&key);
            }
        }
    }

    fn should_defer_startup_child(&self, child: &ExpandedNode) -> bool {
        let _phase = self.startup_phase("mount.classify");
        let state = self.startup.borrow();
        if !state.is_prioritizing() && !(state.background && state.viewport_order) {
            return false;
        }
        drop(state);
        self.startup_demand(child).is_none()
    }

    fn reconsider_startup_deferred(&self) {
        let deferred = {
            let mut state = self.startup.borrow_mut();
            if !state.is_prioritizing() {
                return;
            }
            state.plans.clear();
            state.deferred.clone()
        };
        let mut promoted = Vec::new();
        let mut retained = VecDeque::new();
        for pending in deferred {
            if pending
                .child
                .upgrade()
                .is_some_and(|child| self.should_defer_startup_child(&child))
            {
                retained.push_back(pending);
            } else {
                promoted.push(pending);
            }
        }
        let mut state = self.startup.borrow_mut();
        state.deferred = retained;
        state.stack.extend(promoted.into_iter().rev());
    }

    pub(crate) fn defer_initial_mount(
        &self,
        parent: &Rc<ExpandedNode>,
        child: &Rc<ExpandedNode>,
    ) -> bool {
        let mut startup = self.startup.borrow_mut();
        // Settings providers are a dependency of their consumers, so retain the
        // synchronous provider setup contract inside the parent's local mount.
        if !startup.active || child.is_import_settings_node() {
            return false;
        }
        let key = (
            parent.id,
            parent.mount_epoch.get(),
            child.id,
            child.mount_epoch.get(),
        );
        if startup.background
            && !startup
                .background_parents
                .contains(&(parent.id, parent.mount_epoch.get()))
        {
            return startup.queued.contains(&key);
        }
        if startup.queued.insert(key) {
            startup.enqueue_order = startup
                .enqueue_order
                .checked_add(1)
                .expect("mount enqueue identity exhausted");
            let work = PendingMount {
                arrival: startup.enqueue_order,
                parent: Rc::downgrade(parent),
                child: Rc::downgrade(child),
                parent_epoch: parent.mount_epoch.get(),
                child_epoch: child.mount_epoch.get(),
                key,
            };
            self.visible_groups.borrow_mut().queue(parent, child, &work);

            startup.incoming.push(work);
        }
        true
    }

    fn take_ordinary_mount(&self) -> (Option<PendingMount>, bool) {
        let mut state = self.startup.borrow_mut();
        let incoming = std::mem::take(&mut state.incoming);
        if state.background {
            for child in incoming {
                state.stack.push_front(child);
            }
        } else {
            state.stack.extend(incoming.into_iter().rev());
        }
        if std::mem::take(&mut state.warm_turn) {
            // Charge an unsuccessful priority lookup to one real FIFO mount.
            // Even a continuously moving scroller cannot consume every slice
            // reclassifying the same detached roots.
            let pending = state
                .deferred
                .pop_front()
                .or_else(|| state.stack.pop_back());
            (pending, true)
        } else if state.stack.is_empty() && state.background && state.viewport_order {
            // A changing viewport may invalidate priorities while a pass is
            // running. Consume at least one waiting mount before revisiting
            // that same frontier again.
            state.classifying_frontier = false;
            (state.deferred.pop_front(), true)
        } else {
            (state.stack.pop_back(), false)
        }
    }

    fn take_spatial_mount(
        &self,
        view: &mut crate::viewport_proximity::AdmissionView,
    ) -> Option<PendingMount> {
        let started = view.measuring().then(web_time::Instant::now);
        view.begin_choice(self);
        let (mut queue, mut incoming) = {
            let mut state = self.startup.borrow_mut();
            let mut incoming = std::mem::take(&mut state.incoming);
            // Gradually transfer the old breadth frontier when the user first scrolls.
            for _ in 0..4 {
                let Some(pending) = state.stack.pop_back() else {
                    break;
                };
                incoming.push(pending);
            }
            (std::mem::take(&mut state.spatial), incoming)
        };
        for work in incoming.drain(..) {
            let priority = work
                .parent
                .upgrade()
                .map_or(admission::Priority::Background, |parent| {
                    self.admission_priority(&parent, view)
                });
            queue.push(work, priority);
        }
        // Recheck the provisional winner and one other live owner. A scroll
        // reversal and late layout feedback cannot strand an old classification.
        for owner in [queue.next_to_refresh(), queue.first()]
            .into_iter()
            .flatten()
        {
            if let Some(parent) = self
                .get_expanded_node_by_eid(owner.0)
                .filter(|p| p.mount_epoch.get() == owner.1)
            {
                queue.rerank(owner, self.admission_priority(&parent, view));
            }
        }
        selection_capture::before(self, &queue);
        let pending = self
            .visible_groups
            .borrow_mut()
            .next(self, view, &mut queue)
            .or_else(|| queue.pop());
        selection_capture::after(pending.as_ref());
        self.startup.borrow_mut().spatial = queue;
        if let Some(started) = started {
            let ms = started.elapsed().as_secs_f64() * 1000.;
            if view.first {
                view.stats.first_choices += 1;
                view.stats.first_choice_ms += ms;
            } else {
                view.stats.later_choice_ms += ms;
            }
        }
        view.first = false;
        pending
    }

    fn mount_one_pending(
        self: &Rc<Self>,
        view: Option<&mut crate::viewport_proximity::AdmissionView>,
    ) -> Option<MountOutcome> {
        let (pending, warm) = if let Some(view) = view {
            (self.take_spatial_mount(view), true)
        } else {
            self.take_ordinary_mount()
        };
        let Some(pending) = pending else {
            return None;
        };
        self.apply_pending(pending, warm)
    }
    fn apply_pending(self: &Rc<Self>, pending: PendingMount, warm: bool) -> Option<MountOutcome> {
        let Some(child) = pending.child.upgrade() else {
            self.startup.borrow_mut().queued.remove(&pending.key);
            self.visible_groups.borrow_mut().discard_work(&pending);
            return Some(MountOutcome::Stale);
        };
        let Some(parent) = pending.parent.upgrade() else {
            self.startup.borrow_mut().queued.remove(&pending.key);
            self.visible_groups.borrow_mut().discard_work(&pending);
            return Some(MountOutcome::Stale);
        };
        if pending.runnable(&parent, &child) {
            if !warm && self.should_defer_startup_child(&child) {
                let mut state = self.startup.borrow_mut();
                state.warm_turn = state.background && state.viewport_order;
                state.deferred.push_back(pending);
                return Some(MountOutcome::Deferred);
            }
            {
                let mut state = self.startup.borrow_mut();
                state.queued.remove(&pending.key);
                if state.background {
                    state.background_parents.insert(startup_key(&child));
                    state.awaiting_background_settlement = true;
                }
            }

            self.visible_groups
                .borrow_mut()
                .before_mount(&child, self.startup.borrow().background);

            child.recurse_mount(self);
            parent.finish_pending_child(&child, self);
            selection_capture::mounted(self, &pending, &child);
            return Some(MountOutcome::Mounted(child));
        } else {
            self.startup.borrow_mut().queued.remove(&pending.key);
        }
        self.visible_groups.borrow_mut().discard_work(&pending);
        Some(MountOutcome::Stale)
    }
}

impl PaxEngine {
    /// Enable the experimental initial-mount continuation before mounting a root.
    /// The host must service it to completion before calling ordinary `tick`.
    /// This changes scheduling only; every initially selected node still mounts.
    pub fn enable_resumable_startup(&mut self) {
        assert!(
            self.root_expanded_node.is_none(),
            "enable before mounting the root"
        );
        *self.runtime_context.startup.borrow_mut() = Startup {
            active: true,
            ..Default::default()
        };
    }

    pub fn startup_pending(&self) -> bool {
        self.runtime_context.startup.borrow().active
    }

    /// Enable queue and mount attribution after enabling resumable startup and
    /// before mounting the root. Profiling adds overhead and retains diagnostic
    /// names in both debug and release; it never changes the scheduling budget.
    pub fn enable_startup_profiling(&mut self) {
        assert!(
            self.root_expanded_node.is_none(),
            "enable before mounting the root"
        );
        let mut state = self.runtime_context.startup.borrow_mut();
        assert!(state.active, "enable resumable startup first");
        let profile = StartupProfile::default();
        state.profile_phases = Some(profile.phases.clone());
        state.profile = Some(profile);
        self.runtime_context
            .property_graph
            .set_diagnostic_names_enabled(true);
    }

    /// Consume the completed capture and release diagnostic names. Returns None
    /// while mounting remains pending, when disabled, or after the capture was taken.
    pub fn take_startup_profile(&mut self) -> Option<StartupProfileReport> {
        if self.startup_pending() {
            return None;
        }
        let profile = self.runtime_context.startup.borrow_mut().profile.take()?;
        self.runtime_context.startup.borrow_mut().profile_phases = None;
        if !self.runtime_context.work_profiling_enabled() {
            self.runtime_context.property_graph.set_diagnostic_names_enabled(false);
        }
        Some(profile.into_report())
    }

    /// Prioritize a new viewport checkpoint after host layout/asset feedback.
    /// Park background admission, settle the mounted graph and its required
    /// dependencies, then let the host measure and present before resuming.
    /// Requests coalesce until that checkpoint is acknowledged. This does not
    /// advance application time or replay already-dispatched lifecycle handlers.
    pub fn request_startup_settlement(&mut self) {
        let mut state = self.runtime_context.startup.borrow_mut();
        if !state.active || state.background || state.is_prioritizing() {
            return;
        }
        if let Some(profile) = &mut state.profile {
            profile.settlement_requests += 1;
        }
        state.settling_viewport = true;
        state.ready = false;
        state.phase = Phase::InitialSettlement;
        state.handlers = None;
        state.plans.clear();
        state.demands.clear();
        state.measured.clear();
        state.measurements_ready = false;
        // Preserve the continuation's DFS order, including children emitted by
        // its last mount. Their existing weak identities still guard remounts.
        let incoming = std::mem::take(&mut state.incoming);
        state.stack.extend(incoming.into_iter().rev());
        let stack = std::mem::take(&mut state.stack);
        state.deferred.extend(stack.into_iter().rev());
    }

    /// Validate the next publication boundary. Browser measurements can arrive
    /// after the final service call but before rAF; settle those without replaying
    /// the first tick's handlers or proximity notifications.
    pub fn prepare_startup_presentation(&mut self) -> bool {
        if self.runtime_context.startup.borrow().blocks_presentation() {
            return false;
        }
        // Stage @in before the first visible checkpoint, including when the
        // offscreen remainder is still deferred. Clocks remain held by the host.
        self.prepare_presentation_entry();
        let ctx = &self.runtime_context;
        let _graph = ctx.property_graph.enter();
        ctx.reconsider_startup_deferred();
        let mounts_pending = {
            let state = ctx.startup.borrow();
            !state.stack.is_empty() || !state.incoming.is_empty()
        };
        if mounts_pending
            || ctx.property_graph.has_pending()
            || ctx.settled_surface_blocker().is_some()
        {
            let mut state = ctx.startup.borrow_mut();
            state.active = true;
            state.ready = false;
            state.measurements_ready = false;
            state.phase = Phase::InitialSettlement;
            return false;
        }
        true
    }

    /// Accept the host's quiet font/native-measurement checkpoint before the
    /// next reveal. A flow can now admit its next visible child using resolved
    /// preceding geometry. Returns false when that adds work to service first.
    pub fn accept_startup_measurements(&mut self) -> bool {
        if !self.prepare_startup_presentation() {
            return false;
        }
        let ctx = &self.runtime_context;
        let _graph = ctx.property_graph.enter();
        if !ctx.startup.borrow().is_prioritizing() {
            return true;
        }
        let demands = ctx.startup.borrow().demands.clone();
        let measured = demands
            .into_iter()
            .filter(|((id, epoch), _)| {
                ctx.get_expanded_node_by_eid(*id)
                    .is_some_and(|node| node.mount_epoch.get() == *epoch)
            })
            .collect();
        {
            let mut state = ctx.startup.borrow_mut();
            state.measured = measured;
            state.measurements_ready = true;
        }
        self.prepare_startup_presentation()
    }

    /// Acknowledge a rendered startup checkpoint and eagerly start the next
    /// deferred subtrees. Ordinary ticks remain held until all work ends; the host
    /// can release input separately once its loading screen has exited.
    /// The first acknowledgement must follow the host's font/measurement gate.
    pub fn resume_startup_after_presentation(&mut self) {
        let mut state = self.runtime_context.startup.borrow_mut();
        if !state.ready {
            return;
        }
        if let Some(profile) = &mut state.profile {
            profile.presented_checkpoints += 1;
        }
        state.viewport_published = true;
        state.settling_viewport = false;
        state.plans.clear();
        state.demands.clear();
        state.measured.clear();
        state.measurements_ready = false;
        if !state.active {
            state.resolvers.clear();
            state.measurement_axes.clear();
        }
        if !state.deferred.is_empty() {
            let remaining = std::mem::take(&mut state.deferred);
            state.stack.extend(remaining.into_iter().rev());
            state.ready = false;
            state.phase = Phase::InitialSettlement;
        }
    }

    /// Transfer the remaining mount identities to the ordinary frame loop after
    /// the host has acknowledged the first viewport. Pending nodes stay detached.
    pub fn begin_background_mounting(&mut self) -> bool {
        {
            let state = self.runtime_context.startup.borrow();
            if !state.active || !state.ready || state.viewport_published {
                return false;
            }
        }
        self.resume_startup_after_presentation();
        self.runtime_context
            .visible_groups
            .borrow_mut()
            .first_viewport_published();
        self.runtime_context.startup.borrow_mut().background = true;
        true
    }

    /// Select the cold, retained, or checked priority path in the startup experiment.
    pub fn configure_admission_resumption(&self, mode: u8, measure: bool) {
        let mut state = self.runtime_context.startup.borrow_mut();
        state.resume_mode = mode;
        state.measure_selection = measure;
        state.admission_view = None;
        state.rank_stats = Default::default();
        self.runtime_context.admission_changes.borrow_mut().enabled = mode != 0;
    }

    /// Snapshot cumulative selection work without disturbing queue order.
    pub fn admission_rank_stats(&self) -> crate::viewport_proximity::RankStats {
        self.runtime_context.startup.borrow().rank_stats
    }

    pub fn prioritize_background_owners(&self) {
        let mut state = self.runtime_context.startup.borrow_mut();
        if state.background && !state.viewport_order {
            state.prioritize_owners = true;
        }
    }

    pub fn background_counts(&self) -> [u32; 6] {
        let state = self.runtime_context.startup.borrow();
        let [m, d, s] = state.admission_counts;
        [
            m,
            d,
            s,
            state.incoming.len() as u32,
            (state.stack.len()
                + state.spatial.len
                + state.live_mounts.as_ref().map_or(0, Mounts::len)) as u32,
            state.deferred.len() as u32,
        ]
    }

    pub fn use_viewport_order(&mut self) {
        let mut state = self.runtime_context.startup.borrow_mut();
        state.viewport_order = true;
        state.viewport_changed = true;
    }

    pub fn refresh_background_viewport(&self) {
        let mut state = self.runtime_context.startup.borrow_mut();
        if state.background && state.viewport_order {
            state.viewport_changed = true;
        }
    }

    pub fn admission_can_continue(&self) -> bool {
        let ctx = &self.runtime_context;
        let s = ctx.startup.borrow();
        let _g = ctx.property_graph.enter();
        s.background
            && (s.live_mounts.as_ref().is_some_and(|l| l.len() != 0)
                || s.spatial.len > 0
                || !s.stack.is_empty()
                || !s.incoming.is_empty()
                || !s.deferred.is_empty())
            && properties::pending_effect_count() < BACKGROUND_EFFECT_LIMIT
    }
    pub fn has_indexed_mounts(&self) -> bool {
        let s = self.runtime_context.startup.borrow();
        s.share_viewport_demand
            || (s.accepts_mount_input()
                && s.live_mounts.as_ref().is_some_and(|l| {
                    l.needs_first_observation() || (s.background && s.prioritize_owners)
                }))
    }
    /// Diagnostic existence witness. Stops at the first attached parent whose queued child is legal.
    pub fn has_runnable_background_mount(&self) -> bool {
        let s = self.runtime_context.startup.borrow();
        s.background
            && (s
                .incoming
                .iter()
                .chain(s.stack.iter())
                .chain(s.deferred.iter())
                .chain(s.spatial.all_pending())
                .any(PendingMount::legal_now)
                || s.live_mounts.as_ref().is_some_and(Mounts::any_legal))
    }
    fn service_indexed_frontier(&self, deadline: f64, now: &mut impl FnMut() -> f64) -> bool {
        let ctx = &self.runtime_context;
        let mut live = ctx
            .startup
            .borrow_mut()
            .live_mounts
            .take()
            .expect("active mount frontier");
        live.begin_grant();
        let mounted = live.counts.mounted;
        loop {
            let start = web_time::Instant::now();
            let incoming = {
                let mut state = ctx.startup.borrow_mut();
                state
                    .incoming
                    .pop()
                    .or_else(|| state.stack.pop_back())
                    .or_else(|| state.deferred.pop_front())
            };
            let action = live.step(ctx, incoming);
            let mut maintenance = false;
            match action {
                SelectionUnit::Empty => break,
                SelectionUnit::Maintenance => maintenance = true,
                SelectionUnit::Mount(work) => {
                    let id = work.key.2.to_u32();
                    // Release the scheduler's borrow before running user hooks
                    // or synchronous reads. Attachments append to ordinary ingress.
                    ctx.startup.borrow_mut().live_mounts = Some(live);
                    let t = web_time::Instant::now();
                    let outcome = ctx
                        .apply_pending(work, true)
                        .expect("selected a mount unit");
                    let okay = matches!(outcome, MountOutcome::Mounted(_));
                    let mut s = ctx.startup.borrow_mut();
                    s.admission_counts[if okay { 0 } else { 2 }] += 1;
                    live = s.live_mounts.take().expect("active mount frontier");
                    drop(s);
                    live.finish_mount(
                        ctx,
                        id,
                        okay,
                        t.elapsed().as_micros().min(u32::MAX as u128) as u32,
                    );
                }
            }
            live.counted(start.elapsed().as_secs_f64() * 1000., maintenance);
            if now() >= deadline {
                break;
            }
        }
        if live.counts.mounted > mounted {
            live.counts.grants_with_mount += 1;
        } else {
            live.counts.slices_all_maintenance += 1;
        }
        ctx.startup.borrow_mut().live_mounts = Some(live);
        true
    }
    /// Admit a batch before an ordinary tick. That tick settles its effects and
    /// runs lifecycle handlers on the mounted tree. Return whether more work exists.
    pub fn service_background_mounts(
        &mut self,
        budget_ms: f64,
        mut now_ms: impl FnMut() -> f64,
    ) -> bool {
        let ctx = &self.runtime_context;
        if !ctx.startup.borrow().background {
            return false;
        }
        {
            let mut state = ctx.startup.borrow_mut();
            if std::mem::take(&mut state.viewport_changed) {
                state.plans.clear();
                state.demands.clear();
                state.measured.clear();
                state.measurements_ready = true;
                // Reclassify one pending identity at a time. Never traverse the mounted tree.
                if !state.classifying_frontier {
                    let parked = std::mem::take(&mut state.deferred);
                    state.stack.extend(parked.into_iter().rev());
                    state.classifying_frontier = true;
                }
            }
            if state.live_mounts.as_ref().is_none_or(|l| l.len() == 0)
                && state.spatial.len == 0
                && state.stack.is_empty()
                && state.incoming.is_empty()
                && state.deferred.is_empty()
            {
                if state.awaiting_background_settlement {
                    return true;
                }
                state.background = false;
                state.admission_view = None;
                *ctx.admission_changes.borrow_mut() = Default::default();
                state.active = false;
                state.background_parents.clear();
                state.spatial = Default::default();
                state.resolvers.clear();
                state.measurement_axes.clear();
                return false;
            }
        }
        let deadline = now_ms() + budget_ms;
        let _graph = ctx.property_graph.enter();

        if ctx.startup.borrow().prioritize_owners && ctx.startup.borrow().live_mounts.is_some() {
            return self.service_indexed_frontier(deadline, &mut now_ms);
        }
        let mut view = {
            let mut state = ctx.startup.borrow_mut();
            state.prioritize_owners.then(|| {
                let mut view = state.admission_view.take().unwrap_or_else(|| {
                    let mut v = ctx.new_admission_view(state.resume_mode, state.measure_selection);
                    v.stats = state.rank_stats;
                    v
                });
                view.first = true;
                view
            })
        };
        loop {
            let Some(outcome) = ctx.mount_one_pending(view.as_mut()) else {
                break;
            };
            let index = match outcome {
                MountOutcome::Mounted(_) => 0,
                MountOutcome::Deferred => 1,
                MountOutcome::Stale => 2,
            };
            ctx.startup.borrow_mut().admission_counts[index] += 1;
            if now_ms() >= deadline {
                break;
            }
        }
        if let Some(view) = view {
            let mut state = ctx.startup.borrow_mut();
            state.rank_stats = view.stats;
            if state.resume_mode != 0 {
                state.admission_view = Some(view);
            }
        }
        // The following ordinary tick may enqueue children of the last parent.
        true
    }

    /// Service part of the first logical tick without advancing it once per host
    /// slice. Native patches may be drained between calls for measurement, but
    /// rendering waits for `prepare_startup_presentation`. A ready checkpoint can
    /// still have deferred work; the host resumes it after presenting the scene.
    pub fn service_startup(
        &mut self,
        budget_ms: f64,
        mut now_ms: impl FnMut() -> f64,
    ) -> StartupReport {
        let start = now_ms();
        let mut report = StartupReport::default();
        let ctx = Rc::clone(&self.runtime_context);
        let _graph = ctx.property_graph.enter();
        // No startup-state borrow crosses user callbacks. Labels remain graph-local
        // while the collector is owned by this service call.
        let mut profile = ctx.startup.borrow_mut().profile.take();
        // Import once per service, before local work, to accept measurement and
        // application-store changes without keeping a graph guard across yields.
        ctx.property_graph
            .import_batch_with_observer(1024, &mut |_| {});
        report.peak_queued_effects = properties::pending_effect_count();
        while self.startup_pending()
            && !ctx.startup.borrow().ready
            && (report.units == 0 || now_ms() - start < budget_ms.max(0.0))
        {
            let unit_start = now_ms();
            let phase = ctx.startup.borrow().phase;
            let dispatching_handlers = ctx
                .startup
                .borrow()
                .handlers
                .as_ref()
                .is_some_and(|handlers| !handlers.is_empty());
            let name;
            let mut mounted_node = None;
            let drain_background = {
                let mut state = ctx.startup.borrow_mut();
                if state.is_prioritizing() {
                    state.draining_background_effects = false;
                } else if !dispatching_handlers {
                    let pending = properties::pending_effect_count();
                    if pending == 0 {
                        state.draining_background_effects = false;
                    } else if pending >= BACKGROUND_EFFECT_LIMIT {
                        state.draining_background_effects = true;
                    }
                }
                state.draining_background_effects && !dispatching_handlers
            };
            let drained_background =
                drain_background && drain_startup_effects(&mut profile, &mut now_ms) > 0;
            if drain_background && !drained_background {
                // A queue can contain only entries already settled by reads.
                // Popping them must release admission before the next mount.
                ctx.startup.borrow_mut().draining_background_effects = false;
            }
            // Required settlement precedes each phase. A mounted node can enqueue
            // descendants; reverse each newly emitted batch to preserve DFS order.
            if drained_background {
                report.effects += 1;
                name = "effect";
            } else if let Some(outcome) = ctx.mount_one_pending(None) {
                report.mounted += 1;
                name = "mount";
                if let Some(profile) = &mut profile {
                    match outcome {
                        MountOutcome::Mounted(node) => {
                            profile.mounts_completed += 1;
                            mounted_node = Some(node);
                        }
                        MountOutcome::Deferred => profile.mounts_deferred += 1,
                        MountOutcome::Stale => profile.mounts_stale += 1,
                    }
                }
            } else if !dispatching_handlers && drain_startup_effects(&mut profile, &mut now_ms) > 0
            {
                report.effects += 1;
                name = "effect";
            } else {
                let next = match phase {
                    Phase::InitialSettlement => {
                        name = "initial_settlement";
                        Phase::Imperative
                    }
                    Phase::Imperative => {
                        name = "imperative";
                        if let Some(root) = &self.root_expanded_node {
                            root.recurse_update_mounted(&ctx);
                        }
                        Phase::Async
                    }
                    Phase::Async => {
                        name = "async";
                        // The dispatcher already protects callback lifetimes. Its
                        // current batch remains atomic, just like one user handler.
                        if let Err(error) = ctx
                            .application
                            .dispatcher()
                            .drain(|| ctx.drain_node_effects())
                        {
                            log::error!("startup completion panicked: {error}");
                            self.shutdown(pax_runtime_api::StopReason::HostClosed);
                            break;
                        }
                        Phase::Tick
                    }
                    Phase::Tick | Phase::PreRender => {
                        let tick = matches!(phase, Phase::Tick);
                        name = if tick {
                            "tick_handler"
                        } else {
                            "pre_render_handler"
                        };
                        let node = {
                            let mut state = ctx.startup.borrow_mut();
                            state
                                .handlers
                                .get_or_insert_with(|| {
                                    if tick {
                                        ctx.tick_handler_nodes().into()
                                    } else {
                                        ctx.pre_render_handler_nodes().into()
                                    }
                                })
                                .pop_front()
                        };
                        if let Some(id) = node {
                            if let Some(node) = ctx.get_expanded_node_by_eid(id) {
                                let first = {
                                    let mut state = ctx.startup.borrow_mut();
                                    let visited = if tick {
                                        &mut state.ticked
                                    } else {
                                        &mut state.pre_rendered
                                    };
                                    visited.insert((id, node.mount_epoch.get()))
                                };
                                if first {
                                    node.run_lifecycle_handlers(
                                        if tick {
                                            crate::constants::TICK_HANDLERS
                                        } else {
                                            crate::constants::PRE_RENDER_HANDLERS
                                        },
                                        &ctx,
                                    );
                                }
                            }
                            phase
                        } else {
                            ctx.startup.borrow_mut().handlers = None;
                            if tick {
                                Phase::PreRender
                            } else {
                                Phase::Clock
                            }
                        }
                    }
                    Phase::Clock => {
                        name = "clock_events";
                        let globals = ctx.globals();
                        if !ctx.presentation_held.get() && !ctx.startup.borrow().clock_advanced {
                            ctx.startup.borrow_mut().clock_advanced = true;
                            globals.elapsed_frames.set(globals.elapsed_frames.get() + 1);
                            globals
                                .elapsed_millis
                                .set(super::saturating_u128_to_u64((globals.get_elapsed_millis)()));
                        }
                        ctx.flush_custom_events().unwrap();
                        Phase::Geometry
                    }
                    Phase::Geometry => {
                        name = "geometry";
                        ctx.prepare_scene_geometry();
                        Phase::Proximity
                    }
                    Phase::Proximity => {
                        name = "proximity";
                        ctx.dispatch_viewport_proximity();
                        Phase::Occlusion
                    }
                    Phase::Occlusion => {
                        name = "occlusion";
                        if ctx.take_occlusion_dirty() {
                            if let Some(root) = &self.root_expanded_node {
                                occlusion::update_node_occlusion(root, &ctx);
                            }
                        }
                        Phase::FinalGeometry
                    }
                    Phase::FinalGeometry => {
                        name = "final_geometry";
                        ctx.prepare_scene_geometry();
                        occlusion::update_native_culling(&ctx, self.scroller_tiling_policy);
                        Phase::Complete
                    }
                    Phase::Complete => {
                        name = "complete";
                        // Layout/resize feedback may have moved a deferred box
                        // into view, or a measurement may require its descendants.
                        ctx.reconsider_startup_deferred();
                        let mut state = ctx.startup.borrow_mut();
                        if !state.stack.is_empty() || !state.incoming.is_empty() {
                            state.phase = Phase::InitialSettlement;
                            continue;
                        }
                        state.ready = true;
                        state.active = !state.deferred.is_empty();
                        Phase::Complete
                    }
                };
                ctx.startup.borrow_mut().phase = next;
            }
            let duration = now_ms() - unit_start;
            if let Some(profile) = &mut profile {
                profile.units.entry(name).or_default().record(duration);
                if let Some(node) = mounted_node {
                    profile
                        .mounts
                        .entry(RuntimeContext::startup_node_category(&node))
                        .or_default()
                        .record(duration);
                }
            }
            if duration > report.longest_unit_ms {
                report.longest_unit_ms = duration;
                report.longest_unit = name;
            }
            report.units += 1;
            report.peak_queued_effects = report
                .peak_queued_effects
                .max(properties::pending_effect_count());
        }
        report.pending = self.startup_pending();
        report.ready_to_present = ctx.startup.borrow().ready;
        report.deferred_roots = ctx.startup.borrow().deferred.len();
        report.elapsed_ms = now_ms() - start;
        if let Some(profile) = &mut profile {
            profile.services += 1;
            profile.service_ms += report.elapsed_ms;
        }
        // A failing callback may shut down and unmount the engine during service.
        // Do not resurrect the capture discarded by unmount.
        if self.root_expanded_node.is_some() {
            ctx.startup.borrow_mut().profile = profile;
        }
        report
    }
}

fn drain_startup_effects(
    profile: &mut Option<StartupProfile>,
    now_ms: &mut impl FnMut() -> f64,
) -> usize {
    if let Some(profile) = profile {
        properties::drain_effects_profiled(1, &mut profile.reactive, now_ms)
    } else {
        properties::drain_effects(1)
    }
}

impl Startup {
    pub(crate) fn is_prioritizing(&self) -> bool {
        self.active && (!self.viewport_published || self.settling_viewport)
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn input_scene_published(&self) -> bool {
        !self.active || self.viewport_published
    }

    pub(crate) fn holds_foreground_tick(&self) -> bool {
        self.active && !self.background
    }

    pub(crate) fn blocks_presentation(&self) -> bool {
        self.active && !self.background && !self.ready
    }
}

/// Frozen recipients for application input captured while startup is pending.
/// A remount of the same node ID is a different input lifetime.
pub struct StartupInput {
    pub(crate) recipients: std::collections::HashMap<ExpandedNodeIdentifier, u64>,
    pub(crate) target: Option<ExpandedNodeIdentifier>,
}

/// Restores the input dispatch context even if a callback unwinds.
pub struct StartupInputGuard {
    context: Rc<RuntimeContext>,
    previous: Option<StartupInput>,
}
impl Drop for StartupInputGuard {
    fn drop(&mut self) {
        *self.context.replaying_startup_input.borrow_mut() = self.previous.take();
    }
}
impl RuntimeContext {
    pub fn accepts_startup_input(&self, node: &ExpandedNode) -> bool {
        self.replaying_startup_input
            .borrow()
            .as_ref()
            .is_none_or(|input| {
                node.attached.get() > 0
                    && input.recipients.get(&node.id) == Some(&node.mount_epoch.get())
            })
    }
    pub fn replay_startup_input(self: &Rc<Self>, input: StartupInput) -> StartupInputGuard {
        let previous = self.replaying_startup_input.replace(Some(input));
        StartupInputGuard {
            context: Rc::clone(self),
            previous,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComponentInstance, Handler, HandlerRegistry, InstanceNode, InstantiationArgs};
    use pax_runtime_api::{
        pax_value::{PaxAny, PaxValue},
        Platform, OS,
    };
    use std::cell::RefCell;

    use crate::api::NodeContext;

    thread_local! { static CALLS: RefCell<Vec<(&'static str, u32)>> = RefCell::new(Vec::new()); }
    thread_local! {
        static REPLAY_READS: RefCell<Option<Vec<(u32, (f64, f64))>>> = const { RefCell::new(None) };
    }
    fn mount(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, _: Option<PaxAny>) {
        REPLAY_READS.with_borrow_mut(|reads| {
            if let Some(reads) = reads {
                reads.push((
                    ctx.expanded_node.upgrade().unwrap().id.0,
                    ctx.bounds_self.get(),
                ));
            }
        });

        CALLS.with_borrow_mut(|v| v.push(("mount", ctx.expanded_node.upgrade().unwrap().id.0)));
    }
    fn tick(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, _: Option<PaxAny>) {
        CALLS.with_borrow_mut(|v| v.push(("tick", ctx.expanded_node.upgrade().unwrap().id.0)));
    }
    fn pre_render(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, _: Option<PaxAny>) {
        CALLS
            .with_borrow_mut(|v| v.push(("pre_render", ctx.expanded_node.upgrade().unwrap().id.0)));
    }
    fn pre_unmount(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, _: Option<PaxAny>) {
        CALLS.with_borrow_mut(|v| v.push(("pre_unmount", ctx.expanded_node.upgrade().unwrap().id.0)));
    }
    fn unmount(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, _: Option<PaxAny>) {
        CALLS.with_borrow_mut(|v| v.push(("unmount", ctx.expanded_node.upgrade().unwrap().id.0)));
    }
    fn component(children: Vec<Rc<dyn InstanceNode>>) -> Rc<ComponentInstance> {
        component_with_common(children, crate::CommonPropertiesInit::Default)
    }
    fn component_with_common(
        children: Vec<Rc<dyn InstanceNode>>,
        common: crate::CommonPropertiesInit,
    ) -> Rc<ComponentInstance> {
        component_with_transition(children, common, Default::default())
    }
    fn component_with_transition(
        children: Vec<Rc<dyn InstanceNode>>,
        common: crate::CommonPropertiesInit,
        transition_config: pax_manifest::cartridge_generation::ComponentTransitionConfig,
    ) -> Rc<ComponentInstance> {
        let mut handlers = HandlerRegistry::default();
        for (name, callback) in [
            (
                "mount",
                mount as fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>),
            ),
            ("tick", tick),
            ("pre_render", pre_render),
            ("pre_unmount", pre_unmount),
            ("unmount", unmount),
        ] {
            handlers
                .handlers
                .insert(name.into(), vec![Handler::new_component_handler(callback)]);
        }
        ComponentInstance::instantiate(InstantiationArgs {
            prototypical_common_properties: common,
            prototypical_properties: crate::PropertiesInit::Factory(Box::new(|_, _| {
                Some(Rc::new(RefCell::new(PaxAny::Builtin(PaxValue::default()))))
            })),
            handler_registry: Some(Rc::new(RefCell::new(handlers))),
            children: None,
            component_template: Some(RefCell::new(children)),
            component_settings: None,
            component_camera: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config,
            properties_scope: crate::PropertiesScopeInit::None,
        })
    }
    fn new_engine() -> PaxEngine {
        CALLS.with_borrow_mut(Vec::clear);
        let mut engine = crate::test_support::empty_engine(
            (320., 240.),
            Platform::Web,
            OS::Mac,
            Box::new(|| 10),
            Default::default(),
        );
        engine.enable_resumable_startup();
        engine
    }
    fn finish(engine: &mut PaxEngine) {
        for _ in 0..2000 {
            if !engine.startup_pending() {
                return;
            }
            if engine.accept_startup_measurements() {
                engine.resume_startup_after_presentation();
            }
            let report = engine.service_startup(0., || 0.);
            assert_eq!(
                report.units, 1,
                "one indivisible unit per zero-budget slice"
            );
        }
        panic!("startup did not terminate");
    }

    #[test]
    fn startup_profile_preserves_service_and_lifecycle_work() {
        let run = |profiled| {
            let mut engine = new_engine();
            if profiled {
                engine.enable_startup_profiling();
            }
            engine.mount_root_component(component(vec![
                box_at(0., vec![component(vec![])]),
                box_at(500., vec![component(vec![])]),
            ]));
            assert!(engine.take_startup_profile().is_none());
            let mut totals = (0, 0, 0);
            for _ in 0..4000 {
                if !engine.startup_pending() {
                    break;
                }
                if engine.accept_startup_measurements() {
                    engine.resume_startup_after_presentation();
                }
                let report = engine.service_startup(0., || 0.);
                totals.0 += report.units;
                totals.1 += report.mounted;
                totals.2 += report.effects;
            }
            assert!(!engine.startup_pending());
            let capture = engine.take_startup_profile();
            assert!(!engine
                .runtime_context
                .property_graph
                .diagnostic_names_enabled());
            assert!(engine.take_startup_profile().is_none());
            let calls =
                CALLS.with_borrow(|calls| calls.iter().map(|(kind, _)| *kind).collect::<Vec<_>>());
            (totals, calls, capture)
        };
        let (normal, calls, capture) = run(false);
        assert!(capture.is_none());
        let (profiled, profiled_calls, capture) = run(true);
        assert_eq!(normal, profiled);
        assert_eq!(calls, profiled_calls);
        let capture = capture.unwrap();
        assert_eq!(capture.reactive.totals.evaluations(), profiled.2);
        assert_eq!(
            capture.mounts_completed + capture.mounts_deferred + capture.mounts_stale,
            profiled.1
        );
        assert_eq!(
            capture.units.values().map(|unit| unit.count).sum::<usize>(),
            profiled.0
        );
        assert!(capture.presented_checkpoints > 0);
        assert!(capture.mounts_deferred > 0);
        assert!(!capture.mounts.is_empty());
        assert!(capture.phases["node.create"].count > 0);
        assert!(capture.phases["node.mount"].count > 0);
        assert!(capture.phases.values().all(|phase| phase.self_ms <= phase.total_ms));
    }

    #[test]
    fn startup_profile_unmount_discards_capture_and_allows_retry() {
        let mut engine = new_engine();
        engine.enable_startup_profiling();
        engine.mount_root_component(component(vec![box_at(500., vec![])]));
        engine.service_startup(0., || 0.);
        engine.unmount();
        assert!(engine.take_startup_profile().is_none());
        assert!(!engine
            .runtime_context
            .property_graph
            .diagnostic_names_enabled());

        engine.enable_resumable_startup();
        engine.enable_startup_profiling();
        engine.mount_root_component(component(vec![]));
        finish(&mut engine);
        assert!(engine.take_startup_profile().is_some());
    }

    fn box_at(x: f64, children: Vec<Rc<dyn InstanceNode>>) -> Rc<ComponentInstance> {
        component_with_common(
            children,
            crate::CommonPropertiesInit::Factory(Box::new(move |_, _| {
                Some(Rc::new(RefCell::new(crate::api::CommonProperties {
                    x: pax_runtime_api::LocalProperty::new(Some(pax_runtime_api::Size::Pixels(
                        x.into(),
                    ))),
                    width: pax_runtime_api::LocalProperty::new(Some(
                        pax_runtime_api::Size::Pixels(100.into()),
                    )),
                    height: pax_runtime_api::LocalProperty::new(Some(
                        pax_runtime_api::Size::Pixels(100.into()),
                    )),
                    ..Default::default()
                })))
            })),
        )
    }

    fn reach_checkpoint(engine: &mut PaxEngine) -> StartupReport {
        for _ in 0..2000 {
            let report = engine.service_startup(0.0, || 0.0);
            if engine.accept_startup_measurements() {
                return report;
            }
        }
        panic!("startup did not reach a presentation checkpoint");
    }

    #[test]
    fn background_admission_drains_in_batches_and_feedback_preserves_lifecycles() {
        let mut engine = new_engine();
        let ctx = Rc::clone(&engine.runtime_context);
        let _graph = ctx.property_graph.enter();
        engine.mount_root_component(component(vec![
            box_at(0.0, vec![]),
            box_at(
                500.0,
                (0..32)
                    .map(|_| {
                        component(
                            (0..32)
                                .map(|_| component(vec![]) as Rc<dyn InstanceNode>)
                                .collect(),
                        ) as Rc<dyn InstanceNode>
                    })
                    .collect(),
            ),
        ]));
        reach_checkpoint(&mut engine);
        engine.resume_startup_after_presentation();
        let mut mounted = 0;
        let mut effects = 0;
        let mut peak = 0;
        let mut feedback_start = None;
        let mut feedback_units = None;
        let mut batch_started = false;
        let mut batch_finished = false;
        for unit in 0..300_000 {
            let report = engine.service_startup(0.0, || 0.0);
            mounted += report.mounted;
            effects += report.effects;
            peak = peak.max(report.peak_queued_effects);
            if feedback_start.is_none() {
                if report.effects > 0 {
                    batch_started = true;
                } else if batch_started && report.mounted > 0 {
                    // Finish the admitted batch before another background mount;
                    // alternating single effects/mounts would amplify layout work.
                    assert!(!ctx.startup.borrow().draining_background_effects);
                    batch_finished = true;
                }
            }
            if mounted >= 512 && feedback_start.is_none() {
                engine.request_startup_settlement();
                feedback_start = Some(unit);
            }
            if engine.accept_startup_measurements() {
                if feedback_units.is_none() {
                    feedback_units = feedback_start.map(|start| unit - start);
                }
                engine.resume_startup_after_presentation();
            }
            if !engine.startup_pending() {
                eprintln!("backlog: peak={peak}, mounted={mounted}, effects={effects}, units={}, feedback_units={feedback_units:?}", unit + 1);
                assert!(batch_started && batch_finished);
                assert!(feedback_units.is_some());
                CALLS.with_borrow(|calls| {
                    for kind in ["mount", "tick", "pre_render"] {
                        let ids: Vec<_> = calls
                            .iter()
                            .filter(|(k, _)| *k == kind)
                            .map(|(_, id)| *id)
                            .collect();
                        assert_eq!(ids.len(), 1059);
                        assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 1059);
                    }
                });
                return;
            }
        }
        panic!("measurement did not finish");
    }

    #[test]
    fn clean_queued_entries_release_background_admission() {
        let mut engine = new_engine();
        let ctx = Rc::clone(&engine.runtime_context);
        let _graph = ctx.property_graph.enter();
        let root = engine.mount_root_component(component(vec![box_at(500.0, vec![])]));
        reach_checkpoint(&mut engine);
        engine.resume_startup_after_presentation();
        let _effects: Vec<_> = (0..BACKGROUND_EFFECT_LIMIT)
            .map(|_| {
                let effect = pax_runtime_api::LocalProperty::computed(|| (), &[]);
                properties::register_effect_property(&effect);
                effect.get();
                effect
            })
            .collect();
        assert!(properties::pending_effect_count() >= BACKGROUND_EFFECT_LIMIT);
        let report = engine.service_startup(0.0, || 0.0);
        assert_eq!(report.mounted, 1);
        assert_eq!(report.effects, 0);
        assert_eq!(root.children.get()[0].attached.get(), 1);
        assert!(!ctx.startup.borrow().draining_background_effects);
        finish(&mut engine);
    }

    #[test]
    fn viewport_checkpoint_defers_whole_offscreen_roots_then_eagerly_finishes() {
        let mut engine = new_engine();
        let root = engine.mount_root_component(component(vec![
            box_at(500.0, vec![box_at(-500.0, vec![])]),
            box_at(0.0, vec![]),
            component(vec![]), // unknown dimensions must stay eager
        ]));
        let children = root.children.get();
        let report = reach_checkpoint(&mut engine);
        assert!(report.pending);
        assert_eq!(report.deferred_roots, 1);
        assert_eq!(children[0].attached.get(), 0);
        assert_eq!(children[1].attached.get(), 1);
        assert_eq!(children[2].attached.get(), 1);
        assert_eq!(root.presentation_children(&engine.runtime_context).len(), 2);
        assert!(!engine
            .runtime_context
            .startup
            .borrow()
            .blocks_presentation());
        engine.tick();
        assert_eq!(engine.runtime_context.globals().elapsed_frames.get(), 1);

        engine.resume_startup_after_presentation();
        finish(&mut engine);
        assert_eq!(children[0].attached.get(), 1);
        assert_eq!(children[0].children.get()[0].attached.get(), 1);
        assert_eq!(engine.runtime_context.globals().elapsed_frames.get(), 1);
        CALLS.with_borrow(|calls| {
            for kind in ["mount", "tick", "pre_render"] {
                let ids: Vec<_> = calls
                    .iter()
                    .filter(|(k, _)| *k == kind)
                    .map(|(_, id)| *id)
                    .collect();
                assert_eq!(ids.len(), 5);
                assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 5);
            }
        });
    }

    #[test]
    fn held_viewport_and_later_mounts_stage_entry_without_advancing_the_clock() {
        use pax_manifest::cartridge_generation::{
            ComponentTransitionConfig, TRANSITION_PHASE_ENTER,
        };
        let transition = ComponentTransitionConfig {
            has_enter: true,
            enter_frame_count: 10,
            ..Default::default()
        };
        let deferred = component_with_transition(
            vec![],
            crate::CommonPropertiesInit::Factory(Box::new(|_, _| {
                Some(Rc::new(RefCell::new(crate::api::CommonProperties {
                    x: pax_runtime_api::LocalProperty::new(Some(pax_runtime_api::Size::Pixels(
                        500.into(),
                    ))),
                    width: pax_runtime_api::LocalProperty::new(Some(
                        pax_runtime_api::Size::Pixels(100.into()),
                    )),
                    height: pax_runtime_api::LocalProperty::new(Some(
                        pax_runtime_api::Size::Pixels(100.into()),
                    )),
                    ..Default::default()
                })))
            })),
            transition.clone(),
        );
        let mut engine = new_engine();
        engine.hold_presentation();
        let root = engine.mount_root_component(component_with_transition(
            vec![deferred],
            crate::CommonPropertiesInit::Default,
            transition,
        ));
        let child = root.children.get()[0].clone();
        assert!(reach_checkpoint(&mut engine).pending);
        assert_eq!(root.transition_phase.get(), TRANSITION_PHASE_ENTER);
        assert_eq!(root.transition_playhead.get(), 0.0);
        assert_eq!(child.attached.get(), 0);
        assert_eq!(engine.runtime_context.globals().elapsed_frames.get(), 0);
        engine.resume_startup_after_presentation();
        finish(&mut engine);
        assert_eq!(child.attached.get(), 1);
        assert_eq!(child.transition_phase.get(), TRANSITION_PHASE_ENTER);
        assert_eq!(child.transition_playhead.get(), 0.0);
        assert_eq!(root.transition_playhead.get(), 0.0);
        assert_eq!(engine.runtime_context.globals().elapsed_frames.get(), 0);
        engine.publish_presentation();
        engine.tick();
        assert_eq!(root.transition_playhead.get(), 1.0);
        assert_eq!(child.transition_playhead.get(), 1.0);
    }

    #[test]
    fn required_measurement_promotes_an_offscreen_subtree_before_first_reveal() {
        let mut engine = new_engine();
        let root = engine
            .mount_root_component(component(vec![box_at(500.0, vec![box_at(500.0, vec![])])]));
        assert!(reach_checkpoint(&mut engine).pending);
        root.get_node_context(&engine.runtime_context)
            .set_startup_planner(|_, plan| {
                plan.require_children(StartupDemand::complete());
                // A later visibility request must not weaken the complete dependency.
                plan.require_children(StartupDemand::visible(kurbo::Rect::new(0.0, 0.0, 1.0, 1.0)));
            });
        assert!(!engine.prepare_startup_presentation());
        let report = reach_checkpoint(&mut engine);
        assert!(!report.pending);
        assert_eq!(root.children.get()[0].children.get()[0].attached.get(), 1);
    }

    #[test]
    fn viewport_resize_rechecks_deferred_bounds_before_reveal() {
        let mut engine = new_engine();
        let root = engine.mount_root_component(component(vec![box_at(500.0, vec![])]));
        assert!(reach_checkpoint(&mut engine).pending);
        let viewport = engine.runtime_context.globals().viewport;
        let mut resized = viewport.get();
        resized.bounds = (800.0, 240.0);
        viewport.set(resized);
        assert!(!engine.prepare_startup_presentation());
        assert!(!reach_checkpoint(&mut engine).pending);
        assert_eq!(root.children.get()[0].attached.get(), 1);
    }

    #[test]
    fn post_reveal_feedback_settles_without_draining_the_background_queue() {
        let mut engine = new_engine();
        engine.hold_presentation();
        let root = engine.mount_root_component(component(vec![
            box_at(0.0, vec![]),
            box_at(500.0, vec![box_at(0.0, vec![])]),
            box_at(700.0, vec![]),
            box_at(2000.0, vec![]),
        ]));
        let children = root.children.get();
        reach_checkpoint(&mut engine);
        engine.resume_startup_after_presentation();
        // Interrupt with both a DFS stack and a newly emitted child batch.
        assert_eq!(engine.service_startup(0.0, || 0.0).mounted, 1);
        assert_eq!(children[1].attached.get(), 1);
        engine.set_viewport_size((800.0, 240.0));
        children[0].chassis_resize_request(80.0, 40.0);
        for _ in 0..8 {
            engine.request_startup_settlement();
        }
        let report = reach_checkpoint(&mut engine);
        assert!(report.pending);
        assert_eq!(children[1].children.get()[0].attached.get(), 1);
        assert_eq!(children[2].attached.get(), 1, "newly visible dependency");
        assert_eq!(children[3].attached.get(), 0, "unrelated tail stays parked");
        assert_eq!(engine.runtime_context.globals().elapsed_frames.get(), 0);
        engine.resume_startup_after_presentation();
        finish(&mut engine);
        assert_eq!(children[3].attached.get(), 1);
        assert_eq!(engine.runtime_context.globals().elapsed_frames.get(), 0);
        CALLS.with_borrow(|calls| {
            for kind in ["mount", "tick", "pre_render"] {
                let ids: Vec<_> = calls
                    .iter()
                    .filter(|(k, _)| *k == kind)
                    .map(|(_, id)| *id)
                    .collect();
                assert_eq!(ids.len(), 6);
                assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 6);
            }
        });
    }

    #[test]
    fn feedback_checkpoint_retains_planners_registered_during_background_mounting() {
        let mut engine = new_engine();
        let root = engine.mount_root_component(component(vec![
            box_at(500.0, vec![box_at(1500.0, vec![box_at(1500.0, vec![])])]),
            box_at(2000.0, vec![]),
        ]));
        let children = root.children.get();
        reach_checkpoint(&mut engine);
        engine.resume_startup_after_presentation();
        engine.service_startup(0.0, || 0.0);
        children[0]
            .get_node_context(&engine.runtime_context)
            .set_startup_planner(|_, plan| plan.require_children(StartupDemand::complete()));
        engine.set_viewport_size((800.0, 240.0));
        engine.request_startup_settlement();
        assert!(reach_checkpoint(&mut engine).pending);
        let dependency = &children[0].children.get()[0];
        assert_eq!(dependency.children.get()[0].attached.get(), 1);
        assert_eq!(children[1].attached.get(), 0);
        // A removed queued branch must still fail the existing lifetime checks.
        root.attach_children(
            vec![children[0].clone()],
            &engine.runtime_context,
            &root.parent_frame,
        );
        engine.resume_startup_after_presentation();
        finish(&mut engine);
        assert_eq!(children[1].attached.get(), 0);
    }

    #[test]
    fn feedback_checkpoint_remeasures_an_already_mounted_flow_prefix() {
        let mut engine = new_engine();
        let root = engine.mount_root_component(component(vec![
            box_at(0.0, vec![]),
            box_at(500.0, vec![]),
            box_at(600.0, vec![]),
            box_at(2000.0, vec![]),
        ]));
        let children = root.children.get();
        test_flow(&root.get_node_context(&engine.runtime_context), &children);
        reach_checkpoint(&mut engine);
        engine.resume_startup_after_presentation();
        engine.service_startup(0.0, || 0.0);
        assert_eq!(
            children[1].attached.get(),
            1,
            "background prefix already mounted"
        );
        engine.set_viewport_size((800.0, 240.0));
        engine.request_startup_settlement();
        assert!(reach_checkpoint(&mut engine).pending);
        assert_eq!(children[1].attached.get(), 1);
        assert_eq!(
            children[2].attached.get(),
            1,
            "newly visible child follows the measured prefix"
        );
        assert_eq!(children[3].attached.get(), 0);
        engine.resume_startup_after_presentation();
        finish(&mut engine);
        assert_eq!(children[3].attached.get(), 1);
    }

    fn test_flow(ctx: &crate::api::NodeContext, children: &[Rc<ExpandedNode>]) {
        let children: Vec<_> = children.iter().map(Rc::downgrade).collect();
        ctx.set_startup_planner(move |_, plan| {
            let demand = plan.demand();
            for child in children.iter().filter_map(Weak::upgrade) {
                if plan
                    .bounds(&child)
                    .is_some_and(|b| b.x0 > demand.region.unwrap().x1)
                {
                    break;
                }
                plan.require(
                    &child,
                    demand.merge(StartupDemand::measure(StartupAxes { x: true, y: false })),
                );
                if !plan.measured(&child, demand) {
                    break;
                }
            }
        });
    }

    #[test]
    fn flow_prefix_completes_preceding_measurement_subtrees_but_defers_the_tail() {
        let mut engine = new_engine();
        let root = engine.mount_root_component(component(vec![
            box_at(-500.0, vec![box_at(500.0, vec![])]),
            box_at(0.0, vec![box_at(500.0, vec![])]),
            box_at(500.0, vec![box_at(500.0, vec![])]),
        ]));
        let children = root.children.get();
        test_flow(&root.get_node_context(&engine.runtime_context), &children);
        let report = reach_checkpoint(&mut engine);
        assert!(report.pending);
        assert_eq!(report.deferred_roots, 1);
        for child in &children[..2] {
            assert_eq!(child.attached.get(), 1);
            assert_eq!(child.children.get()[0].attached.get(), 1);
        }
        assert_eq!(children[2].attached.get(), 0);
        engine.resume_startup_after_presentation();
        finish(&mut engine);
        assert_eq!(children[2].children.get()[0].attached.get(), 1);
    }
    #[test]
    fn flow_waits_for_native_feedback_before_admitting_a_provisionally_visible_sibling() {
        let mut engine = new_engine();
        let root = engine
            .mount_root_component(component(vec![box_at(0.0, vec![]), box_at(150.0, vec![])]));
        let children = root.children.get();
        test_flow(&root.get_node_context(&engine.runtime_context), &children);
        for _ in 0..2000 {
            if engine.service_startup(0.0, || 0.0).ready_to_present {
                break;
            }
        }
        assert!(engine.prepare_startup_presentation());
        assert_eq!(children[0].attached.get(), 1);
        assert_eq!(children[1].attached.get(), 0);
        // Model a native text response growing the preceding flow child, moving
        // the next child's provisional box below/beyond the viewport.
        children[1]
            .get_common_properties()
            .borrow()
            .x
            .set(Some(pax_runtime_api::Size::Pixels(500.into())));
        assert!(!engine.prepare_startup_presentation());
        assert!(reach_checkpoint(&mut engine).pending);
        assert_eq!(children[1].attached.get(), 0);
        engine.resume_startup_after_presentation();
        finish(&mut engine);
        assert_eq!(children[1].attached.get(), 1);
    }

    #[test]
    fn yields_preserve_depth_first_mount_order_and_one_logical_tick() {
        let mut engine = new_engine();
        let root = engine.mount_root_component(component(vec![
            component(vec![component(vec![])]),
            component(vec![]),
        ]));
        assert!(root
            .presentation_children(&engine.runtime_context)
            .is_empty());
        assert_eq!(
            root.children.get().len(),
            2,
            "pending children remain structurally visible"
        );
        assert_eq!(engine.runtime_context.globals().elapsed_frames.get(), 0);
        engine.tick();
        assert_eq!(
            engine.runtime_context.globals().elapsed_frames.get(),
            0,
            "ordinary ticks must not bypass startup"
        );
        finish(&mut engine);
        let calls = CALLS.with_borrow(Clone::clone);
        let mounts: Vec<_> = calls
            .iter()
            .filter(|(phase, _)| *phase == "mount")
            .map(|(_, id)| *id)
            .collect();
        assert_eq!(mounts, vec![0, 1, 3, 2]);
        for phase in ["tick", "pre_render"] {
            let mut ids: Vec<_> = calls
                .iter()
                .filter(|(p, _)| *p == phase)
                .map(|(_, id)| *id)
                .collect();
            ids.sort();
            assert_eq!(ids, vec![0, 1, 2, 3]);
        }
        assert_eq!(root.current_attached_children().len(), 2);
        assert_eq!(engine.runtime_context.globals().elapsed_frames.get(), 1);
    }
    #[test]
    fn removed_pending_branch_never_mounts_and_unmount_cancels_work() {
        let mut engine = new_engine();
        let root =
            engine.mount_root_component(component(vec![component(vec![]), component(vec![])]));
        let children = root.children.get();
        root.attach_children(
            vec![children[1].clone()],
            &engine.runtime_context,
            &root.parent_frame,
        );
        finish(&mut engine);
        assert_eq!(children[0].attached.get(), 0);
        assert_eq!(children[1].attached.get(), 1);
        assert!(CALLS.with_borrow(|c| c.iter().all(|(_, id)| *id != children[0].id.0)));
        CALLS.with_borrow_mut(Vec::clear);
        root.attach_children(Vec::new(), &engine.runtime_context, &root.parent_frame);
        assert_eq!(
            CALLS.with_borrow(Clone::clone),
            [("pre_unmount", children[1].id.0), ("unmount", children[1].id.0)],
            "teardown finishes in the attachment call"
        );
        let mut another = new_engine();
        another.mount_root_component(component(vec![component(vec![])]));
        another.unmount();
        assert!(!another.startup_pending());
    }
    #[test]
    fn late_feedback_settles_before_publication_without_repeating_lifecycle() {
        let mut engine = new_engine();
        let root = engine.mount_root_component(component(vec![]));
        finish(&mut engine);
        let calls = CALLS.with_borrow(Clone::clone);
        let clock = engine.runtime_context.globals().elapsed_frames.get();
        root.chassis_resize_request(80., 40.);
        assert!(!engine.prepare_startup_presentation());
        for _ in 0..10 {
            finish(&mut engine);
            if engine.prepare_startup_presentation() {
                break;
            }
        }
        assert!(engine.prepare_startup_presentation());
        assert_eq!(engine.runtime_context.globals().elapsed_frames.get(), clock);
        assert_eq!(CALLS.with_borrow(Clone::clone), calls);
    }

    #[test]
    fn pending_slot_reparenting_uses_the_current_attachment() {
        let mut engine = new_engine();
        let root =
            engine.mount_root_component(component(vec![component(vec![]), component(vec![])]));
        engine.service_startup(0., || 0.);
        engine.service_startup(0., || 0.);
        let parents = root.presentation_children(&engine.runtime_context);
        assert_eq!(parents.len(), 2);
        let child = root
            .create_children_detached(
                [(
                    component(vec![]) as Rc<dyn InstanceNode>,
                    root.stack.clone(),
                )],
                &engine.runtime_context,
                &Rc::downgrade(&root),
            )
            .remove(0);
        parents[0].attach_children(
            vec![child.clone()],
            &engine.runtime_context,
            &parents[0].parent_frame,
        );
        parents[1].attach_children(
            vec![child.clone()],
            &engine.runtime_context,
            &parents[1].parent_frame,
        );
        finish(&mut engine);
        assert_eq!(child.attached.get(), 1);
        assert!(parents[0].current_attached_children().is_empty());
        assert_eq!(parents[1].current_attached_children()[0].id, child.id);
        assert_eq!(
            CALLS.with_borrow(|c| c
                .iter()
                .filter(|entry| **entry == ("mount", child.id.0))
                .count()),
            1
        );
    }

    #[test]
    fn queued_input_targets_the_revealed_viewport_before_full_mount() {
        struct HitTarget(crate::BaseInstance);
        impl InstanceNode for HitTarget {
            fn instantiate(args: InstantiationArgs) -> Rc<Self> {
                Rc::new(Self(crate::BaseInstance::new(
                    args,
                    crate::InstanceFlags {
                        invisible_to_slot: false,
                        invisible_to_raycasting: false,
                        layer: crate::api::Layer::Canvas,
                        is_component: false,
                        is_slot: false,
                    },
                )))
            }
            fn base(&self) -> &crate::BaseInstance {
                &self.0
            }
            fn resolve_debug(
                &self,
                f: &mut std::fmt::Formatter,
                _: Option<&ExpandedNode>,
            ) -> std::fmt::Result {
                f.write_str("HitTarget")
            }
        }
        let target = HitTarget::instantiate(InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Default,
            prototypical_properties: crate::PropertiesInit::Factory(Box::new(|_, _| {
                Some(Rc::new(RefCell::new(PaxAny::Builtin(PaxValue::default()))))
            })),
            handler_registry: None,
            children: None,
            component_template: None,
            component_settings: None,
            component_camera: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        });
        let mut engine = new_engine();
        engine.hold_presentation();
        let root = engine.mount_root_component(component(vec![
            box_at(0., vec![target]),
            box_at(500., vec![]),
        ]));
        let ctx = Rc::clone(&engine.runtime_context);
        assert!(reach_checkpoint(&mut engine).pending);
        let ray = Some(pax_runtime_api::math::Point2::new(10., 10.));
        assert!(ctx.capture_startup_input(ray).target.is_none());
        engine.resume_startup_after_presentation();
        assert!(engine.startup_pending());
        let snapshot = ctx.capture_startup_input(ray);
        let target = snapshot
            .target
            .expect("a published viewport accepts queued clicks");
        let pending = root.children.get()[1].clone();
        assert_eq!(pending.attached.get(), 0);
        finish(&mut engine);
        let _guard = ctx.replay_startup_input(snapshot);
        assert_eq!(
            ctx.get_topmost_element_beneath_ray(ray.unwrap()).unwrap().id,
            target
        );
        assert!(!ctx.accepts_startup_input(&pending));
        assert_eq!(ctx.globals().elapsed_frames.get(), 0);
    }

    #[test]
    fn input_before_an_offscreen_subscription_mounts_is_observed_as_current_state() {
        use crate::api::{ButtonClick, Event, LocalProperty, Store};

        struct Selection {
            value: LocalProperty<u32>,
            observations: Rc<RefCell<Vec<u32>>>,
        }
        impl Store for Selection {}

        let mut engine = new_engine();
        engine.hold_presentation();
        let button = box_at(0., vec![]);
        let subscriber = box_at(500., vec![]);
        let bind = |node: &ComponentInstance, name: &str, handler: Handler| {
            node.base()
                .handler_registry
                .as_ref()
                .unwrap()
                .borrow_mut()
                .handlers
                .insert(name.to_owned(), vec![handler]);
        };
        bind(
            &button,
            "button_click",
            Handler::new_component_handler(|_, ctx, _| {
                ctx.with_store::<Selection, _>(|s| s.value.set(s.value.get() + 1))
                    .unwrap();
            }),
        );
        bind(
            &subscriber,
            "mount",
            Handler::new_component_handler(|_, ctx, _| {
                let (value, observations) = ctx
                    .with_store::<Selection, _>(|s| (s.value.clone(), s.observations.clone()))
                    .unwrap();
                ctx.subscribe(&[value.untyped()], move || {
                    observations.borrow_mut().push(value.get())
                });
            }),
        );
        let root = engine.mount_root_component(component(vec![button, subscriber]));
        let ctx = Rc::clone(&engine.runtime_context);
        let _graph = ctx.property_graph.enter();
        let value = LocalProperty::new(0);
        let observations = Rc::new(RefCell::new(Vec::new()));
        root.get_node_context(&ctx)
            .provide_store(Selection {
                value: value.clone(),
                observations: observations.clone(),
            })
            .unwrap();
        assert!(reach_checkpoint(&mut engine).pending);
        engine.resume_startup_after_presentation();
        let children = root.children.get();
        assert_eq!(children[1].attached.get(), 0);
        let click = || {
            children[0].dispatch_button_click(Event::new(ButtonClick {}), &ctx.globals(), &ctx);
            ctx.drain_node_effects();
        };
        click();
        click();
        assert_eq!(value.get(), 2);
        assert!(observations.borrow().is_empty());

        finish(&mut engine);
        assert_eq!(&*observations.borrow(), &[2]);
        click();
        assert_eq!(&*observations.borrow(), &[2, 3]);
        children[1].clone().recurse_unmount(&ctx);
        click();
        assert_eq!(value.get(), 4);
        assert_eq!(&*observations.borrow(), &[2, 3]);
    }

    #[test]
    fn frozen_input_excludes_new_nodes_and_remounted_lifetimes() {
        let mut engine = new_engine();
        let root = engine.mount_root_component(component(vec![component(vec![])]));
        let ctx = Rc::clone(&engine.runtime_context);
        let snapshot = ctx.capture_startup_input(None);
        finish(&mut engine);
        let child = root.current_attached_children()[0].clone();
        let guard = ctx.replay_startup_input(snapshot);
        assert!(ctx.accepts_startup_input(&root));
        assert!(!ctx.accepts_startup_input(&child));
        root.clone().recurse_unmount(&ctx);
        root.recurse_mount(&ctx);
        assert!(!ctx.accepts_startup_input(&root));
        drop(guard);
        assert!(ctx.accepts_startup_input(&root));
    }
    #[test]
    fn visible_group_releases_only_its_own_mounted_lifetime() {
        struct Output<const NATIVE: bool>(crate::BaseInstance);
        impl<const NATIVE: bool> InstanceNode for Output<NATIVE> {
            fn instantiate(args: InstantiationArgs) -> Rc<Self> {
                Rc::new(Self(crate::BaseInstance::new(
                    args,
                    crate::InstanceFlags {
                        invisible_to_slot: false,
                        invisible_to_raycasting: false,
                        layer: if NATIVE {
                            crate::api::Layer::Native
                        } else {
                            crate::api::Layer::Canvas
                        },
                        is_component: false,
                        is_slot: false,
                    },
                )))
            }
            fn base(&self) -> &crate::BaseInstance {
                &self.0
            }
            fn awaits_native_first_content(&self) -> bool {
                NATIVE
            }
            fn resolve_debug(
                &self,
                f: &mut std::fmt::Formatter,
                _: Option<&ExpandedNode>,
            ) -> std::fmt::Result {
                f.write_str("TestOutput")
            }
        }
        let args = || InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Default,
            prototypical_properties: crate::PropertiesInit::Factory(Box::new(|_, _| {
                Some(Rc::new(RefCell::new(PaxAny::Builtin(PaxValue::default()))))
            })),
            handler_registry: None,
            children: None,
            component_template: None,
            component_settings: None,
            component_camera: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        };
        let shell = |x, selected: bool| {
            use pax_runtime_api::pax_value::ToFromPaxAny;
            let mut branch = args();
            branch.children = Some(RefCell::new(vec![
                Output::<true>::instantiate(args()),
                Output::<false>::instantiate(args()),
            ]));
            branch.prototypical_properties =
                crate::PropertiesInit::Factory(Box::new(move |_, existing| {
                    existing.is_none().then(|| {
                        Rc::new(RefCell::new(
                            crate::ConditionalProperties {
                                boolean_expression: pax_runtime_api::LocalProperty::new(selected),
                                conditional_branches: vec![],
                            }
                            .to_pax_any(),
                        ))
                    })
                }));
            box_at(x, vec![crate::ConditionalInstance::instantiate(branch)])
        };
        let mut engine = new_engine();
        let ctx = Rc::clone(&engine.runtime_context);
        ctx.configure_visible_groups(true);
        engine.hold_presentation();
        REPLAY_READS.with_borrow_mut(|r| *r = Some(Vec::new()));
        let root = engine.mount_root_component(component(vec![
            box_at(0., vec![]),
            shell(1000., true),
            shell(2000., true),
            shell(3000., false),
        ]));
        assert!(reach_checkpoint(&mut engine).pending);
        assert!(engine.begin_background_mounting());
        engine.publish_presentation();
        for _ in 0..64 {
            let mut clock = 0.;
            let more = engine.service_background_mounts(2., || {
                clock += 1.;
                clock
            });
            let ordinary = root.current_attached_children();
            for (id, _, resources) in ctx.ready_visible_groups() {
                let node = ordinary.iter().find(|n| n.id.to_u32() == id).unwrap();
                let pax_runtime_api::Size::Pixels(px) =
                    node.get_common_properties().borrow().x.get().unwrap()
                else {
                    unreachable!()
                };
                let x = px.to_float();
                if x < 2900. {
                    assert_eq!(
                        resources.len(),
                        1,
                        "selected lazy child was still undiscovered"
                    );
                }
            }
            engine.tick();
            if !more {
                break;
            }
        }
        let _graph = ctx.property_graph.enter();
        let ready = ctx.ready_visible_groups();
        assert_eq!(ready.len(), 3);
        assert_eq!(ready[2].2.len(), 0);
        let (a, ae, ar) = &ready[0];
        let (b, be, br) = &ready[1];
        assert_eq!((ar.len(), br.len()), (1, 1));
        assert!(
            ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(ar[0]))
                .unwrap()
                .attached
                .get()
                > 0
        );
        assert_eq!(root.current_attached_children().len(), 4);
        assert_eq!(root.mounted_presentation_children(&ctx).len(), 1);
        assert!(!ctx.open_visible_group(*a, ae.wrapping_add(1)));
        assert!(ctx.open_visible_group(*a, *ae));
        assert!(!ctx.open_visible_group(*a, *ae));
        assert_eq!(root.mounted_presentation_children(&ctx).len(), 2);
        assert_eq!(ctx.hidden_native_group_members(&[ar[0], br[0]]), [br[0]]);
        let reads = REPLAY_READS.with_borrow_mut(|r| r.take().unwrap());
        for id in [a, b] {
            assert_eq!(
                reads
                    .iter()
                    .filter(|r| r.0 == *id && r.1 == (100., 100.))
                    .count(),
                1
            );
        }
        let (inactive, initial_publication, _) = ready[2];
        assert!(ctx.open_visible_group(inactive, initial_publication));
        let conditional = ctx
            .get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(inactive))
            .unwrap()
            .current_attached_children()[0]
            .clone();
        let condition =
            conditional.with_properties_unwrapped(|p: &mut crate::ConditionalProperties| {
                p.boolean_expression.clone()
            });
        condition.set(true);
        let first = conditional.children.get();
        assert_eq!(first.len(), 2);
        assert!(first.iter().all(|n| n.attached.get() == 1));
        assert_eq!(first[0].transform_and_bounds.get().bounds, (100., 100.));
        engine.tick();
        let selected = |ctx: &RuntimeContext| {
            ctx.ready_visible_groups()
                .into_iter()
                .find(|(id, _, _)| *id == conditional.id.to_u32())
                .unwrap()
        };
        let (_, old, old_resources) = selected(&ctx);
        assert_eq!(old_resources, [first[0].id.to_u32()]);
        assert_eq!(
            ctx.hidden_native_group_members(&old_resources),
            old_resources
        );
        condition.set(false);
        assert!(conditional.children.get().is_empty());
        assert!(first.iter().all(|n| n.attached.get() == 0));
        condition.set(true);
        let replacement = conditional.children.get();
        engine.tick();
        let (id, current, resources) = selected(&ctx);
        assert_ne!(current, old);
        assert!(!ctx.open_visible_group(id, old));
        assert_eq!(resources, [replacement[0].id.to_u32()]);
        assert!(ctx.open_visible_group(id, current));
        assert!(ctx.hidden_native_group_members(&resources).is_empty());
        engine.unmount();
        assert!(!ctx.open_visible_group(*b, *be));
        assert!(ctx.ready_visible_groups().is_empty());
    }
    #[test]
    fn foreground_ticks_while_initial_mounts_are_parked_then_admitted() {
        let mut engine = new_engine();
        engine.hold_presentation();
        let root = engine.mount_root_component(component(vec![
            box_at(0., vec![]),
            box_at(500., vec![component(vec![])]),
        ]));
        assert!(reach_checkpoint(&mut engine).pending);
        assert!(engine.accept_startup_measurements());
        assert!(engine.begin_background_mounting());
        let parked = root.children.get()[1].clone();
        engine.publish_presentation();
        for frame in 1..=3 {
            engine.tick();
            assert_eq!(parked.attached.get(), 0);
            assert_eq!(engine.runtime_context.globals().elapsed_frames.get(), frame);
        }
        assert!(engine.service_background_mounts(0., || 0.));
        engine.tick();
        assert_eq!(parked.attached.get(), 1);
        for _ in 0..20 {
            if !engine.service_background_mounts(0., || 0.) {
                break;
            }
            engine.tick();
        }
        assert!(!engine.startup_pending());
        assert_eq!(parked.children.get()[0].attached.get(), 1);
        CALLS.with_borrow(|calls| {
            assert_eq!(
                calls
                    .iter()
                    .filter(|&&(kind, id)| kind == "mount" && id == parked.id.0)
                    .count(),
                1
            );
        });
    }

    #[test]
    fn indexed_mount_dispatch_preserves_hook_lifetimes_and_synchronous_bounds_across_yields() {
        fn run(indexed: bool) -> (Vec<(u32, (f64, f64))>, Vec<u32>) {
            let mut engine = new_engine();
            engine.hold_presentation();
            REPLAY_READS.with_borrow_mut(|r| *r = Some(Vec::new()));
            let root = engine.mount_root_component(component(
                (0..7)
                    .map(|i| {
                        box_at(
                            400. * i as f64,
                            vec![box_at(0., vec![]), box_at(2., vec![])],
                        ) as Rc<dyn InstanceNode>
                    })
                    .collect(),
            ));
            assert!(reach_checkpoint(&mut engine).pending);
            assert!(engine.begin_background_mounting());
            engine.configure_admission_resumption(1, false);
            if indexed {
                engine.enable_indexed_mounts(0.);
                engine.observe_mount_hosts(
                    0.,
                    [0., 0., 300., 300.],
                    &Default::default(),
                    &[],
                    250.,
                );
            }
            engine.prioritize_background_owners();
            engine.publish_presentation();
            for turn in 0..600 {
                if indexed && turn == 10 {
                    engine.observe_mount_hosts(
                        10.,
                        [600., 0., 900., 300.],
                        &Default::default(),
                        &[],
                        250.,
                    );
                }
                let more = engine.service_background_mounts(0., || 0.);
                engine.tick();
                if !more {
                    break;
                }
            }
            assert!(!engine.startup_pending());
            if indexed {
                let state = engine.runtime_context.startup.borrow();
                let counts = &state.live_mounts.as_ref().unwrap().counts;
                assert!(counts.mounted > 0 && counts.slices_all_maintenance > 0);
            }
            let hooks = CALLS.with_borrow(|v| {
                v.iter()
                    .filter(|p| p.0 == "mount")
                    .map(|p| p.1)
                    .collect::<Vec<_>>()
            });
            let mut todo = vec![root.clone()];
            while let Some(parent) = todo.pop() {
                for n in parent.children.get() {
                    assert_eq!(n.attached.get(), 1);
                    let before = hooks.iter().position(|&id| id == parent.id.0).unwrap();
                    let after = hooks.iter().position(|&id| id == n.id.0).unwrap();
                    assert!(before < after);
                    todo.push(n);
                }
            }
            let mut ids: Vec<_> = hooks.into_iter().map(|id| id - root.id.0).collect();
            ids.sort_unstable();
            assert!(ids.windows(2).all(|i| i[0] != i[1]));
            let mut reads = REPLAY_READS.with_borrow_mut(|r| r.take().unwrap());
            for r in &mut reads {
                r.0 -= root.id.0;
            }
            reads.sort_by_key(|r| r.0);
            (reads, ids)
        }
        assert_eq!(run(false), run(true));
    }

    #[test]
    fn indexed_mount_discards_a_replaced_branch_and_drains_its_new_lifetime() {
        let mut engine = new_engine();
        engine.hold_presentation();
        let root = engine.mount_root_component(component(
            (0..8)
                .map(|i| box_at(3000. + 400. * i as f64, vec![]) as Rc<dyn InstanceNode>)
                .collect(),
        ));
        assert!(reach_checkpoint(&mut engine).pending);
        assert!(engine.begin_background_mounting());
        engine.enable_indexed_mounts(0.);
        engine.observe_mount_hosts(0., [0., 0., 320., 240.], &Default::default(), &[], 250.);
        engine.prioritize_background_owners();
        engine.publish_presentation();
        let mut children = root.children.get();
        let retired = children[3].clone();
        for _ in 0..40 {
            if engine
                .runtime_context
                .startup
                .borrow()
                .live_mounts
                .as_ref()
                .unwrap()
                .len()
                == children.len()
            {
                break;
            }
            assert!(engine.service_background_mounts(0., || 0.));
            engine.tick();
        }
        assert_eq!(
            engine
                .runtime_context
                .startup
                .borrow()
                .live_mounts
                .as_ref()
                .unwrap()
                .len(),
            children.len()
        );
        assert_eq!(retired.attached.get(), 0);
        let replacement = root
            .create_children_detached(
                [(
                    box_at(10., vec![]) as Rc<dyn InstanceNode>,
                    root.stack.clone(),
                )],
                &engine.runtime_context,
                &Rc::downgrade(&root),
            )
            .remove(0);
        children[3] = replacement.clone();
        root.attach_children(children, &engine.runtime_context, &root.parent_frame);
        for _ in 0..600 {
            if !engine.service_background_mounts(0., || 0.) {
                break;
            }
            engine.tick();
        }
        assert!(!engine.startup_pending());
        assert_eq!(retired.attached.get(), 0);
        assert_eq!(replacement.attached.get(), 1);
        let active = root.current_attached_children();
        assert_eq!(active.len(), 8);
        assert!(!active.iter().any(|n| n.id == retired.id));
        CALLS.with_borrow(|calls| {
            assert!(!calls.contains(&("mount", retired.id.0)));
            assert_eq!(
                calls
                    .iter()
                    .filter(|(kind, id)| *kind == "mount" && *id == replacement.id.0)
                    .count(),
                1
            );
        });
        engine.unmount();
        assert!(!engine.startup_pending());
    }

    #[test]
    fn retained_admission_replays_the_same_mounts_and_hooks_as_fresh_choices() {
        let run = |mode| {
            let mut engine = new_engine();
            engine.hold_presentation();
            let branches = (0..8)
                .map(|i| {
                    let leaves = (0..4)
                        .map(|_| box_at(0., vec![]) as Rc<dyn InstanceNode>)
                        .collect();
                    box_at(400. * i as f64, leaves) as Rc<dyn InstanceNode>
                })
                .collect();
            let root = engine.mount_root_component(component(branches));
            assert!(reach_checkpoint(&mut engine).pending);
            assert!(engine.begin_background_mounting());
            engine.configure_admission_resumption(mode, false);
            engine.prioritize_background_owners();
            engine.publish_presentation();
            let mut checkpoints = Vec::new();
            for turn in 0..256 {
                // Exactly one attempt makes the cold mode fresh at every choice.
                if [4, 9, 13].contains(&turn) {
                    let child = root.children.get()[1].clone();
                    let x = if turn == 9 { 720. } else { 15. };
                    child
                        .get_common_properties()
                        .borrow()
                        .x
                        .set(Some(pax_runtime_api::Size::Pixels(x.into())));
                }
                let more = engine.service_background_mounts(0., || 0.);
                engine.tick();
                checkpoints.push(CALLS.with_borrow(|c| c.len()));
                if !more {
                    break;
                }
            }
            assert!(!engine.startup_pending());
            let stats = engine.admission_rank_stats();
            assert!(stats.choices > 10);
            if mode == 2 {
                assert_eq!(stats.choices, stats.checked_choices);
            }
            engine.unmount();
            assert!(!engine.runtime_context.admission_changes.borrow().enabled);
            let mut calls = CALLS.with_borrow(|c| c.clone());
            for (_, id) in &mut calls {
                *id -= root.id.to_u32();
            }
            (calls, checkpoints)
        };
        assert_eq!(run(0), run(2));
    }

    #[test]
    fn admission_priority_refreshes_current_geometry_after_each_yield() {
        use admission::Priority;
        let mut engine = new_engine();
        let root = engine.mount_root_component(component(vec![
            box_at(10., vec![]),
            box_at(500., vec![]),
            box_at(1200., vec![]),
        ]));
        finish(&mut engine);
        let ctx = &engine.runtime_context;
        let _graph = ctx.property_graph.enter();
        let children = root.current_attached_children();
        let mut view = ctx.admission_view();
        let ranks = children
            .iter()
            .map(|n| ctx.admission_priority(n, &mut view))
            .collect::<Vec<_>>();
        assert_eq!(
            ranks,
            [Priority::Visible, Priority::Nearby, Priority::Background]
        );
        children[0]
            .get_common_properties()
            .borrow()
            .x
            .set(Some(pax_runtime_api::Size::Pixels(1400.into())));
        children[2]
            .get_common_properties()
            .borrow()
            .x
            .set(Some(pax_runtime_api::Size::Pixels(10.into())));
        let mut view = ctx.admission_view();
        assert_eq!(
            ctx.admission_priority(&children[0], &mut view),
            Priority::Background
        );
        assert_eq!(
            ctx.admission_priority(&children[2], &mut view),
            Priority::Visible
        );
    }
}
