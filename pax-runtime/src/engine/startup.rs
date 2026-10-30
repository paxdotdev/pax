//! Experimental, host-serviced initial mounting. A continuation owns identities
//! and phase cursors only; no property evaluation or borrow crosses a host yield.
use super::occlusion;
pub mod plan;
use crate::{ExpandedNode, ExpandedNodeIdentifier, PaxEngine, RuntimeContext};
use pax_runtime_api::properties;
pub use plan::{StartupAxes, StartupDemand, StartupPlan};
use plan::{StartupRequirements, StartupResolver};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    rc::{Rc, Weak},
};

#[derive(Default)]
pub(crate) struct Startup {
    active: bool,
    ready: bool,
    viewport_published: bool,
    incoming: Vec<PendingMount>,
    stack: Vec<PendingMount>,
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
struct PendingMount {
    parent: Weak<ExpandedNode>,
    child: Weak<ExpandedNode>,
    parent_epoch: u64,
    child_epoch: u64,
    key: (ExpandedNodeIdentifier, u64, ExpandedNodeIdentifier, u64),
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
    pub effects: usize,
    pub elapsed_ms: f64,
    pub longest_unit_ms: f64,
    pub longest_unit: &'static str,
}

impl RuntimeContext {
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
        let tab = node.transform_and_bounds.get();
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
        if state.is_prioritizing() {
            let key = startup_key(node);
            if state.measurement_axes.insert(key, axes) != Some(axes) {
                state.plans.remove(&key);
            }
        }
    }

    fn should_defer_startup_child(&self, child: &ExpandedNode) -> bool {
        if self.startup.borrow().viewport_published {
            return false;
        }
        self.startup_demand(child).is_none()
    }

    fn reconsider_startup_deferred(&self) {
        let deferred = {
            let mut state = self.startup.borrow_mut();
            if state.viewport_published {
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
        if startup.queued.insert(key) {
            startup.incoming.push(PendingMount {
                parent: Rc::downgrade(parent),
                child: Rc::downgrade(child),
                parent_epoch: parent.mount_epoch.get(),
                child_epoch: child.mount_epoch.get(),
                key,
            });
        }
        true
    }

    fn mount_one_pending(self: &Rc<Self>) -> bool {
        let pending = {
            let mut state = self.startup.borrow_mut();
            let incoming = std::mem::take(&mut state.incoming);
            state.stack.extend(incoming.into_iter().rev());
            state.stack.pop()
        };
        let Some(pending) = pending else {
            return false;
        };
        let Some(child) = pending.child.upgrade() else {
            self.startup.borrow_mut().queued.remove(&pending.key);
            return true;
        };
        let Some(parent) = pending.parent.upgrade() else {
            self.startup.borrow_mut().queued.remove(&pending.key);
            return true;
        };
        if child.attached.get() == 0
            && parent.mount_epoch.get() == pending.parent_epoch
            && child.mount_epoch.get() == pending.child_epoch
            && parent.accepts_pending_child(&child)
            && child
                .render_parent_node()
                .is_some_and(|p| Rc::ptr_eq(&p, &parent))
        {
            if self.should_defer_startup_child(&child) {
                self.startup.borrow_mut().deferred.push_back(pending);
                return true;
            }
            self.startup.borrow_mut().queued.remove(&pending.key);
            child.recurse_mount(self);
            parent.finish_pending_child(&child, self);
        } else {
            self.startup.borrow_mut().queued.remove(&pending.key);
        }
        true
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

    /// Validate the next publication boundary. Browser measurements can arrive
    /// after the final service call but before rAF; settle those without replaying
    /// the first tick's handlers or proximity notifications.
    pub fn prepare_startup_presentation(&mut self) -> bool {
        if self.runtime_context.startup.borrow().blocks_presentation() {
            return false;
        }
        if !self.startup_pending() {
            self.prepare_presentation_entry();
        }
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
            state.phase = Phase::InitialSettlement;
            return false;
        }
        true
    }

    /// Accept the host's quiet font/native-measurement checkpoint before the
    /// first reveal. A flow can now admit its next visible child using resolved
    /// preceding geometry. Returns false when that adds work to service first.
    pub fn accept_startup_measurements(&mut self) -> bool {
        if !self.prepare_startup_presentation() {
            return false;
        }
        let ctx = &self.runtime_context;
        let _graph = ctx.property_graph.enter();
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
            if state.viewport_published {
                return true;
            }
            state.measured = measured;
        }
        self.prepare_startup_presentation()
    }

    /// Acknowledge a rendered startup checkpoint and eagerly start the next
    /// deferred subtrees. Input and ordinary ticks remain held until all work ends.
    /// The first acknowledgement must follow the host's font/measurement gate.
    pub fn resume_startup_after_presentation(&mut self) {
        let mut state = self.runtime_context.startup.borrow_mut();
        if !state.ready {
            return;
        }
        state.viewport_published = true;
        state.plans.clear();
        state.demands.clear();
        state.measured.clear();
        state.resolvers.clear();
        state.measurement_axes.clear();
        if !state.deferred.is_empty() {
            let remaining = std::mem::take(&mut state.deferred);
            state.stack.extend(remaining.into_iter().rev());
            state.ready = false;
            state.phase = Phase::InitialSettlement;
        }
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
        // Import once per service, before local work, to accept measurement and
        // application-store changes without keeping a graph guard across yields.
        ctx.property_graph
            .import_batch_with_observer(1024, &mut |_| {});
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
            // Required settlement precedes each phase. A mounted node can enqueue
            // descendants; reverse each newly emitted batch to preserve DFS order.
            if ctx.mount_one_pending() {
                report.mounted += 1;
                name = "mount";
            } else if !dispatching_handlers && properties::drain_effects(1) > 0 {
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
            if duration > report.longest_unit_ms {
                report.longest_unit_ms = duration;
                report.longest_unit = name;
            }
            report.units += 1;
        }
        report.pending = self.startup_pending();
        report.ready_to_present = ctx.startup.borrow().ready;
        report.deferred_roots = ctx.startup.borrow().deferred.len();
        report.elapsed_ms = now_ms() - start;
        report
    }
}

impl Startup {
    pub(crate) fn is_prioritizing(&self) -> bool {
        self.active && !self.viewport_published
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn blocks_presentation(&self) -> bool {
        self.active && !self.ready
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
    fn mount(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, _: Option<PaxAny>) {
        CALLS.with_borrow_mut(|v| v.push(("mount", ctx.expanded_node.upgrade().unwrap().id.0)));
    }
    fn tick(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, _: Option<PaxAny>) {
        CALLS.with_borrow_mut(|v| v.push(("tick", ctx.expanded_node.upgrade().unwrap().id.0)));
    }
    fn pre_render(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, _: Option<PaxAny>) {
        CALLS
            .with_borrow_mut(|v| v.push(("pre_render", ctx.expanded_node.upgrade().unwrap().id.0)));
    }
    fn component(children: Vec<Rc<dyn InstanceNode>>) -> Rc<ComponentInstance> {
        component_with_common(children, crate::CommonPropertiesInit::Default)
    }
    fn component_with_common(
        children: Vec<Rc<dyn InstanceNode>>,
        common: crate::CommonPropertiesInit,
    ) -> Rc<ComponentInstance> {
        let mut handlers = HandlerRegistry::default();
        for (name, callback) in [
            (
                "mount",
                mount as fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>),
            ),
            ("tick", tick),
            ("pre_render", pre_render),
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
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
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
        assert!(!CALLS.with_borrow(|c| c.contains(&("mount", children[0].id.0))));
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
}
