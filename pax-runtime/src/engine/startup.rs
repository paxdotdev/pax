//! Experimental, host-serviced initial mounting. A continuation owns identities
//! and phase cursors only; no property evaluation or borrow crosses a host yield.
use super::occlusion;
use crate::{ExpandedNode, ExpandedNodeIdentifier, PaxEngine, RuntimeContext};
use pax_runtime_api::properties;
use std::{
    collections::{HashSet, VecDeque},
    rc::{Rc, Weak},
};

#[derive(Default)]
pub(crate) struct Startup {
    active: bool,
    incoming: Vec<PendingMount>,
    stack: Vec<PendingMount>,
    queued: HashSet<(ExpandedNodeIdentifier, u64, ExpandedNodeIdentifier, u64)>,
    phase: Phase,
    handlers: Option<VecDeque<ExpandedNodeIdentifier>>,
}

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
    FeedbackGeometry,
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
    pub units: usize,
    /// Mount continuations attempted, including stale entries discarded on service.
    pub mounted: usize,
    pub effects: usize,
    pub elapsed_ms: f64,
    pub longest_unit_ms: f64,
    pub longest_unit: &'static str,
}

impl RuntimeContext {
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
        self.startup.borrow_mut().queued.remove(&pending.key);
        let Some(child) = pending.child.upgrade() else {
            return true;
        };
        let Some(parent) = pending.parent.upgrade() else {
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
            child.recurse_mount(self);
            parent.finish_pending_child(&child, self);
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

    /// Validate the initial publication boundary. Browser measurements can arrive
    /// after the final service call but before rAF; settle those without replaying
    /// the first tick's handlers or proximity notifications.
    pub fn prepare_startup_presentation(&mut self) -> bool {
        if self.startup_pending() {
            return false;
        }
        let ctx = &self.runtime_context;
        let _graph = ctx.property_graph.enter();
        if ctx.property_graph.has_pending() || ctx.settled_surface_blocker().is_some() {
            let mut state = ctx.startup.borrow_mut();
            state.active = true;
            state.phase = Phase::FeedbackGeometry;
            return false;
        }
        true
    }

    /// Service part of the first logical tick without advancing it once per host
    /// slice. Native patches may be drained between calls for measurement, but
    /// rendering and text reveal must wait for `pending == false`.
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
        while self.startup_pending() && (report.units == 0 || now_ms() - start < budget_ms.max(0.0))
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
                                node.run_lifecycle_handlers(
                                    if tick {
                                        crate::constants::TICK_HANDLERS
                                    } else {
                                        crate::constants::PRE_RENDER_HANDLERS
                                    },
                                    &ctx,
                                );
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
                        globals.elapsed_frames.set(globals.elapsed_frames.get() + 1);
                        globals
                            .elapsed_millis
                            .set(super::saturating_u128_to_u64((globals.get_elapsed_millis)()));
                        ctx.flush_custom_events().unwrap();
                        Phase::Geometry
                    }
                    Phase::Geometry => {
                        name = "geometry";
                        ctx.prepare_scene_geometry();
                        Phase::Proximity
                    }
                    Phase::FeedbackGeometry => {
                        name = "feedback_geometry";
                        ctx.prepare_scene_geometry();
                        Phase::Occlusion
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
                        ctx.startup.borrow_mut().active = false;
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
        report.elapsed_ms = now_ms() - start;
        report
    }
}

impl Startup {
    pub(crate) fn is_active(&self) -> bool {
        self.active
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
            prototypical_common_properties: crate::CommonPropertiesInit::Default,
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
            let report = engine.service_startup(0., || 0.);
            assert_eq!(
                report.units, 1,
                "one indivisible unit per zero-budget slice"
            );
        }
        panic!("startup did not terminate");
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
