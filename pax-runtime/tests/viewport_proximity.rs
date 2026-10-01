#![cfg(not(feature = "designtime"))]

use kurbo::{Affine, Rect, Shape};
use pax_runtime::api::*;
use pax_runtime::*;
use pax_runtime_api::pax_value::{PaxAny, ToFromPaxAny};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

#[derive(Clone, Debug)]
struct Sample {
    id: u32,
    kind: &'static str,
    previous: Option<ViewportProximitySnapshot>,
    current: ViewportProximitySnapshot,
}
thread_local! {
    static EVENTS: RefCell<Vec<Sample>> = const { RefCell::new(Vec::new()) };
    static MOVE_ON_ENTER: Cell<bool> = const { Cell::new(false) };
    static UNMOUNT_ON_ENTER: RefCell<Option<Rc<RuntimeContext>>> = const { RefCell::new(None) };
    static OCCLUSION_SAMPLES: RefCell<Vec<(u32, f64)>> = const { RefCell::new(Vec::new()) };
    static TRACK_OCCLUSION: Cell<bool> = const { Cell::new(false) };
}
fn record(
    ctx: &NodeContext,
    kind: &'static str,
    previous: Option<ViewportProximitySnapshot>,
    current: ViewportProximitySnapshot,
) {
    EVENTS.with(|events| {
        events.borrow_mut().push(Sample {
            id: ctx.expanded_node.upgrade().unwrap().id.to_u32(),
            kind,
            previous,
            current,
        })
    });
}
fn enter(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, args: Option<PaxAny>) {
    let event = Event::<ViewportProximityEnter>::from_pax_any(args.unwrap()).unwrap();
    record(ctx, "enter", None, event.current);
    if MOVE_ON_ENTER.with(|flag| flag.replace(false)) {
        ctx.expanded_node
            .upgrade()
            .unwrap()
            .get_common_properties()
            .borrow()
            .y
            .set(Some(Size::Pixels(900.0.into())));
    }
    if let Some(runtime) = UNMOUNT_ON_ENTER.with(|r| r.borrow_mut().take()) {
        ctx.expanded_node
            .upgrade()
            .unwrap()
            .recurse_unmount(&runtime);
    }
}
fn change(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, args: Option<PaxAny>) {
    let event = Event::<ViewportProximityChange>::from_pax_any(args.unwrap()).unwrap();
    record(ctx, "change", event.previous, event.current);
}
fn exit(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, args: Option<PaxAny>) {
    let event = Event::<ViewportProximityExit>::from_pax_any(args.unwrap()).unwrap();
    record(ctx, "exit", Some(event.previous), event.current);
}
fn events() -> Vec<Sample> {
    EVENTS.with(|v| std::mem::take(&mut *v.borrow_mut()))
}
fn kinds(events: &[Sample]) -> Vec<&'static str> {
    events.iter().map(|e| e.kind).collect()
}

fn args(bounds: Rect, events: &[&str]) -> InstantiationArgs {
    let handlers = events
        .iter()
        .map(|&key| {
            let function = match key {
                "viewport_proximity_enter" => enter,
                "viewport_proximity_change" => change,
                "viewport_proximity_exit" => exit,
                _ => unreachable!(),
            };
            (
                key.to_string(),
                vec![Handler::new_component_handler(function)],
            )
        })
        .collect::<HashMap<_, _>>();
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(CommonProperties {
                x: LocalProperty::new(Some(Size::Pixels(bounds.x0.into()))),
                y: LocalProperty::new(Some(Size::Pixels(bounds.y0.into()))),
                width: LocalProperty::new(Some(Size::Pixels(bounds.width().into()))),
                height: LocalProperty::new(Some(Size::Pixels(bounds.height().into()))),
                ..Default::default()
            })))
        })),
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, node| {
            node.is_none()
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
        })),
        handler_registry: (!handlers.is_empty())
            .then(|| Rc::new(RefCell::new(HandlerRegistry { handlers }))),
        children: None,
        component_template: None,
        component_settings: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
        properties_scope: PropertiesScopeInit::None,
    }
}
const TRIO: &[&str] = &[
    "viewport_proximity_enter",
    "viewport_proximity_change",
    "viewport_proximity_exit",
];
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Group,
    Clip,
    Scroll,
    UnclippedScroll,
    Paint,
    Native,
}
struct Probe {
    base: BaseInstance,
    kind: Kind,
}
impl Probe {
    fn new(
        kind: Kind,
        bounds: Rect,
        handlers: &[&str],
        children: Vec<Rc<dyn InstanceNode>>,
    ) -> Rc<Self> {
        let mut args = args(bounds, handlers);
        args.children = Some(RefCell::new(children));
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    layer: if kind == Kind::Paint {
                        Layer::Canvas
                    } else if kind == Kind::Native {
                        Layer::Native
                    } else {
                        Layer::DontCare
                    },
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    is_component: false,
                    is_slot: false,
                },
            ),
            kind,
        })
    }
}
impl InstanceNode for Probe {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    layer: Layer::DontCare,
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    is_component: false,
                    is_slot: false,
                },
            ),
            kind: Kind::Group,
        })
    }
    fn base(&self) -> &BaseInstance {
        &self.base
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Probe").finish()
    }
    fn scrolls_content(&self, _: &ExpandedNode) -> bool {
        matches!(self.kind, Kind::Scroll | Kind::UnclippedScroll)
    }
    fn clips_content(&self, _: &ExpandedNode) -> bool {
        matches!(self.kind, Kind::Scroll | Kind::Clip)
    }
    fn resolve_effect_clip_path(&self, node: &ExpandedNode) -> Option<kurbo::BezPath> {
        self.clips_content(node).then(|| {
            let tab = node.transform_and_bounds.get();
            Affine::from(tab.transform)
                * Rect::new(0.0, 0.0, tab.bounds.0, tab.bounds.1).to_path(0.1)
        })
    }
    fn prepare_canvas_geometry(&self, node: &ExpandedNode) -> scene_geometry::CanvasGeometry {
        scene_geometry::CanvasGeometry::for_layout(node.transform_and_bounds.get().bounds)
    }
    fn resolve_coverage_path(&self, node: &ExpandedNode) -> Option<kurbo::BezPath> {
        if !TRACK_OCCLUSION.with(Cell::get) {
            return None;
        }
        let tab = node.transform_and_bounds.get();
        OCCLUSION_SAMPLES.with(|samples| {
            samples
                .borrow_mut()
                .push((node.id.to_u32(), tab.transform.coeffs()[5]));
        });
        Some(
            Affine::from(tab.transform)
                * Rect::new(0.0, 0.0, tab.bounds.0, tab.bounds.1).to_path(0.1),
        )
    }
    fn handle_mount(self: Rc<Self>, node: &Rc<ExpandedNode>, ctx: &Rc<RuntimeContext>) {
        let children: Vec<_> = self
            .base
            .get_instance_children()
            .borrow()
            .iter()
            .cloned()
            .map(|child| (child, node.stack.clone()))
            .collect();
        let frame = if matches!(self.kind, Kind::Scroll | Kind::UnclippedScroll | Kind::Clip) {
            LocalProperty::new(Some(node.id))
        } else {
            node.parent_frame.clone()
        };
        let children = node.generate_children(children, ctx, &frame, true);
        node.children.set(children);
        if self.kind == Kind::Paint {
            let weak = Rc::downgrade(node);
            let ctx = ctx.clone();
            node.changed_listener.replace_with(LocalProperty::computed(
                move || {
                    if let Some(node) = weak.upgrade() {
                        ctx.mark_canvas_node_dirty(node.id);
                    }
                },
                &[node.transform_and_bounds.untyped()],
            ));
        }
    }
}
fn rect(y: f64) -> Rect {
    Rect::new(0.0, y, 100.0, y + 100.0)
}
fn mount(children: Vec<Rc<dyn InstanceNode>>) -> (PaxEngine, Rc<ExpandedNode>) {
    mount_on(children, Platform::Web)
}
fn mount_on(
    children: Vec<Rc<dyn InstanceNode>>,
    platform: Platform,
) -> (PaxEngine, Rc<ExpandedNode>) {
    events();
    let mut engine = PaxEngine::new_empty(
        (320.0, 240.0),
        platform,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let mut root_args = args(Rect::new(0.0, 0.0, 320.0, 240.0), &[]);
    root_args.component_template = Some(RefCell::new(children));
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    (engine, root)
}

#[test]
fn native_culling_reuses_index_and_restores_on_scroll_geometry_and_topology_changes() {
    use std::collections::HashSet;
    fn apply(messages: Vec<pax_message::NativeMessage>, cold: &mut HashSet<u32>) -> usize {
        let mut changed = 0;
        for message in messages {
            if let pax_message::NativeMessage::NativeCullUpdate(patch) = message {
                changed += patch.cull.len() + patch.restore.len();
                cold.extend(patch.cull);
                for id in patch.restore {
                    cold.remove(&id);
                }
            }
        }
        changed
    }
    let children = (0..1000)
        .map(|i| {
            Probe::new(Kind::Native, rect(i as f64 * 200.0), &[], vec![]) as Rc<dyn InstanceNode>
        })
        .collect();
    let (mut engine, root) = mount_on(
        vec![Probe::new(
            Kind::Scroll,
            Rect::new(0.0, 0.0, 320.0, 240.0),
            &[],
            children,
        )],
        Platform::Native,
    );
    let mut cold = HashSet::new();
    apply(engine.tick(), &mut cold);
    let scroller = root.children.get()[0].clone();
    let natives = scroller.children.get();
    scroll(&engine.runtime_context, &scroller, 0.0, 0.0);
    apply(engine.tick(), &mut cold);
    assert_eq!(cold.len(), 996); // 240px viewport + 512px warm padding
    assert!(!cold.contains(&natives[0].id.to_u32()));
    assert!(cold.contains(&natives[10].id.to_u32()));
    assert_eq!(apply(engine.tick(), &mut cold), 0);
    let before = engine.runtime_context.scene_geometry_stats();
    scroll(&engine.runtime_context, &scroller, 0.0, 2000.0);
    let changes = apply(engine.tick(), &mut cold);
    let after = engine.runtime_context.scene_geometry_stats();
    assert!(changes < 20);
    assert_eq!(after.index_updates, before.index_updates);
    assert!(after.candidate_tests - before.candidate_tests < 30);
    assert!(cold.contains(&natives[0].id.to_u32()));
    assert!(!cold.contains(&natives[10].id.to_u32()));
    // A previously cold native leaf can animate into the warm window.
    y(&natives[500], 2100.0);
    apply(engine.tick(), &mut cold);
    assert!(!cold.contains(&natives[500].id.to_u32()));
    y(&natives[500], 100000.0);
    apply(engine.tick(), &mut cold);
    assert!(cold.contains(&natives[500].id.to_u32()));
    // Disabling scroll islands must restore every retained native view.
    engine
        .runtime_context
        .globals()
        .browser_allows_scroller_vector_layers
        .set(false);
    engine.runtime_context.mark_occlusion_dirty();
    apply(engine.tick(), &mut cold);
    assert!(cold.is_empty());
}

#[test]
fn native_culling_does_not_change_web_or_unclipped_scrollers() {
    for (platform, kind) in [
        (Platform::Web, Kind::Scroll),
        (Platform::Native, Kind::UnclippedScroll),
    ] {
        let (mut engine, root) = mount_on(
            vec![Probe::new(
                kind,
                Rect::new(0.0, 0.0, 320.0, 240.0),
                &[],
                vec![Probe::new(Kind::Native, rect(20000.0), &[], vec![])],
            )],
            platform,
        );
        engine.tick();
        scroll(&engine.runtime_context, &root.children.get()[0], 0.0, 0.0);
        assert!(!engine
            .tick()
            .iter()
            .any(|m| matches!(m, pax_message::NativeMessage::NativeCullUpdate(_))));
    }
}
fn y(node: &ExpandedNode, value: f64) {
    node.get_common_properties()
        .borrow()
        .y
        .set(Some(Size::Pixels(value.into())));
}
fn scroll(ctx: &RuntimeContext, node: &ExpandedNode, x: f64, y: f64) {
    ctx.set_scroller_surface_state(
        node.id.to_u32(),
        ScrollerSurfaceState {
            viewport_width: 320.0,
            viewport_height: 240.0,
            content_width: 100000.0,
            content_height: 100000.0,
            scroll_x: x,
            scroll_y: y,
            presentation_scroll_x: x,
            presentation_scroll_y: y,
            clip_content: true,
        },
    );
}

#[test]
fn initial_regions_first_pixel_terminal_pair_and_reentry() {
    let (mut engine, root) = mount(vec![Probe::new(Kind::Group, rect(300.0), TRIO, vec![])]);
    engine.tick();
    let node = root.children.get()[0].clone();
    let initial = events();
    assert_eq!(kinds(&initial), ["enter", "change"]);
    assert_eq!(initial[0].current, initial[1].current);
    assert!(!initial[0].current.is_in_viewport());
    y(&node, 240.0);
    engine.tick();
    let contact = events();
    assert!(!contact[0].current.is_in_viewport());
    y(&node, 239.0);
    engine.tick();
    let entry = events();
    assert_eq!(
        entry[0].current.viewport_intersection.unwrap().height(),
        1.0
    );
    y(&node, 0.0);
    engine.tick();
    events();
    y(&node, -20.0);
    engine.tick();
    let departing = events();
    assert_eq!(
        departing[0].current.viewport_intersection.unwrap().height(),
        80.0
    );
    assert_eq!(
        departing[0]
            .previous
            .unwrap()
            .viewport_intersection
            .unwrap()
            .height(),
        100.0
    );
    y(&node, 900.0);
    engine.tick();
    let outside = events();
    assert_eq!(kinds(&outside), ["change", "exit"]);
    assert_eq!(outside[0].current, outside[1].current);
    assert!(!outside[0].current.in_proximity);
    y(&node, 0.0);
    engine.tick();
    let again = events();
    assert_eq!(kinds(&again), ["enter", "change"]);
    assert!(again[1].previous.is_none());
}

#[test]
fn cold_jumps_idle_and_enter_only_need_no_handler_coordination() {
    let (mut engine, root) = mount(vec![Probe::new(
        Kind::Group,
        rect(900.0),
        &[TRIO[0]],
        vec![],
    )]);
    engine.tick();
    assert!(events().is_empty());
    let node = root.children.get()[0].clone();
    y(&node, -900.0);
    engine.tick();
    assert!(events().is_empty());
    y(&node, 0.0);
    engine.tick();
    assert_eq!(kinds(&events()), ["enter"]);
    let stats = engine.runtime_context.viewport_proximity_stats();
    for _ in 0..5 {
        engine.tick();
    }
    assert_eq!(engine.runtime_context.viewport_proximity_stats(), stats);
    assert!(events().is_empty());
}

#[test]
fn callback_geometry_is_frozen_until_the_next_batch() {
    MOVE_ON_ENTER.with(|flag| flag.set(true));
    let (mut engine, root) = mount(vec![Probe::new(Kind::Group, rect(0.0), TRIO, vec![])]);
    engine.tick();
    let first = events();
    assert_eq!(kinds(&first), ["enter", "change"]);
    assert_eq!(first[0].current, first[1].current);
    assert_eq!(
        root.children.get()[0]
            .transform_and_bounds
            .get()
            .transform
            .coeffs()[5],
        900.0
    );
    engine.tick();
    assert_eq!(kinds(&events()), ["change", "exit"]);
}

#[test]
fn compositing_runs_once_with_settled_callback_geometry() {
    MOVE_ON_ENTER.with(|flag| flag.set(true));
    let (mut engine, root) = mount(vec![Probe::new(
        Kind::Scroll,
        Rect::new(0.0, 0.0, 320.0, 240.0),
        &[],
        vec![Probe::new(Kind::Paint, rect(0.0), TRIO, vec![])],
    )]);
    OCCLUSION_SAMPLES.with(|samples| samples.borrow_mut().clear());
    TRACK_OCCLUSION.with(|flag| flag.set(true));
    engine.tick();
    TRACK_OCCLUSION.with(|flag| flag.set(false));
    let first = events();
    assert_eq!(kinds(&first), ["enter", "change"]);
    assert_eq!(first[0].current, first[1].current);
    assert_eq!(first[0].current.bounds.y0, 0.0);

    let leaf = root.children.get()[0].children.get()[0].clone();
    // Compositing sees only the final position, while this tick's events retain
    // the pre-handler sample. Initial layer ownership must also settle for paint.
    OCCLUSION_SAMPLES.with(|samples| {
        assert_eq!(*samples.borrow(), [(leaf.id.to_u32(), 900.0)]);
    });
    let ctx = &engine.runtime_context;
    let geometry = ctx.canvas_geometry_for_node(&leaf);
    assert_eq!(geometry.layer, 1);
    assert_eq!(geometry.world_transform.as_coeffs()[5], 900.0);
    assert_eq!(
        ctx.canvas_nodes_intersecting(1, &[rect(900.0)]).unwrap(),
        [leaf.id.to_u32()]
    );
    engine.tick();
    assert_eq!(kinds(&events()), ["change", "exit"]);
}

#[test]
fn unmount_cancels_remaining_delivery_and_suspension_resets_the_baseline() {
    let (mut engine, _) = mount(vec![Probe::new(Kind::Group, rect(0.0), TRIO, vec![])]);
    UNMOUNT_ON_ENTER.with(|runtime| *runtime.borrow_mut() = Some(engine.runtime_context.clone()));
    engine.tick();
    assert_eq!(kinds(&events()), ["enter"]);
    engine.tick();
    assert!(events().is_empty());
    let (mut engine, root) = mount(vec![Probe::new(Kind::Group, rect(0.0), TRIO, vec![])]);
    engine.tick();
    events();
    let node = root.children.get()[0].clone();
    node.get_common_properties()
        .borrow()
        ._suspended
        .set(Some(true));
    engine.tick();
    assert!(events().is_empty());
    node.get_common_properties()
        .borrow()
        ._suspended
        .set(Some(false));
    engine.tick();
    let resumed = events();
    assert_eq!(kinds(&resumed), ["enter", "change"]);
    assert!(resumed[1].previous.is_none());
    node.recurse_unmount(&engine.runtime_context);
    engine.tick();
    assert!(events().is_empty());
}

#[test]
fn disjoint_clips_stay_empty_and_unclippable_can_escape() {
    let leaf = Probe::new(Kind::Group, rect(0.0), TRIO, vec![]);
    let inner = Probe::new(Kind::Clip, rect(150.0), &[], vec![leaf]);
    let outer = Probe::new(Kind::Clip, rect(100.0), &[], vec![inner]);
    let (mut engine, root) = mount(vec![outer]);
    engine.tick();
    assert!(events().is_empty());
    let leaf = root.children.get()[0].children.get()[0].children.get()[0].clone();
    leaf.get_common_properties()
        .borrow()
        .unclippable
        .set(Some(true));
    engine.tick();
    let escaped = events();
    assert_eq!(kinds(&escaped), ["enter", "change"]);
    assert!(escaped[0].current.in_proximity);
    assert!(!escaped[0].current.is_in_viewport());
}

#[test]
fn nested_native_scrolling_and_page_viewport_use_presented_coordinates() {
    let leaf = Probe::new(
        Kind::Paint,
        Rect::new(400.0, 0.0, 500.0, 100.0),
        TRIO,
        vec![],
    );
    let inner = Probe::new(
        Kind::Scroll,
        Rect::new(0.0, 400.0, 320.0, 500.0),
        &[],
        vec![leaf],
    );
    let outer = Probe::new(
        Kind::Scroll,
        Rect::new(0.0, 0.0, 320.0, 240.0),
        &[],
        vec![inner],
    );
    let (mut engine, root) = mount(vec![outer]);
    engine.tick();
    events();
    let outer = root.children.get()[0].clone();
    let inner = outer.children.get()[0].clone();
    let leaf = inner.children.get()[0].clone();
    let ctx = engine.runtime_context.clone();
    scroll(&ctx, &outer, 0.0, 400.0);
    scroll(&ctx, &inner, 400.0, 0.0);
    engine.tick();
    let visible = events();
    assert_eq!(
        visible.last().unwrap().current.viewport_intersection,
        Some(Rect::new(0.0, 0.0, 100.0, 100.0))
    );
    let geometry = ctx.canvas_geometry_for_node(&leaf);
    let before = ctx.viewport_proximity_stats();
    ctx.update_scroller_surface_scroll(outer.id.to_u32(), 0.0, 410.0, 0.0, 410.0);
    engine.tick();
    assert!(Rc::ptr_eq(&geometry, &ctx.canvas_geometry_for_node(&leaf)));
    assert_eq!(
        ctx.viewport_proximity_stats().index_updates,
        before.index_updates
    );
    assert_eq!(
        events()[0].current.viewport_intersection.unwrap().height(),
        90.0
    );

    let leaf = Probe::new(Kind::Group, rect(1050.0), TRIO, vec![]);
    let outer = Probe::new(
        Kind::Scroll,
        Rect::new(0.0, 0.0, 320.0, 240.0),
        &[],
        vec![leaf],
    );
    let (mut engine, root) = mount(vec![outer]);
    engine.tick();
    assert!(events().is_empty());
    let outer = root.children.get()[0].clone();
    engine
        .runtime_context
        .set_root_scroller_id(Some(outer.id.to_u32()));
    engine
        .runtime_context
        .set_visual_viewport_state(VisualViewportState {
            width: 320.0,
            height: 240.0,
            page_scroll_y: 1000.0,
            ..Default::default()
        });
    engine.tick();
    let page = events();
    assert_eq!(kinds(&page), ["enter", "change"]);
    assert_eq!(page[0].current.bounds, rect(50.0));
    assert!(page[0].current.is_in_viewport());
}

#[test]
fn zero_size_resize_transform_and_unsubscribed_targets() {
    let observed = Probe::new(
        Kind::Group,
        Rect::new(0.0, 300.0, 100.0, 300.0),
        TRIO,
        vec![],
    );
    let (mut engine, root) = mount(vec![
        observed,
        Probe::new(Kind::Group, rect(0.0), &[], vec![]),
    ]);
    engine.tick();
    assert!(events().is_empty());
    let node = root.children.get()[0].clone();
    node.get_common_properties()
        .borrow()
        .height
        .set(Some(Size::Pixels(100.0.into())));
    engine.tick();
    assert_eq!(kinds(&events()), ["enter", "change"]);
    engine.set_viewport_size((320.0, 400.0));
    engine.tick();
    assert!(events()[0].current.is_in_viewport());
    node.get_common_properties()
        .borrow()
        .rotate
        .set(Some(Rotation::Degrees(30.0.into())));
    engine.tick();
    assert_eq!(kinds(&events()), ["change"]);
    let (mut engine, _) = mount(vec![Probe::new(Kind::Paint, rect(0.0), &[], vec![])]);
    for _ in 0..4 {
        engine.tick();
    }
    assert_eq!(
        engine.runtime_context.viewport_proximity_stats(),
        Default::default()
    );
}

#[test]
fn scroll_query_cost_tracks_nearby_rows_not_collection_size() {
    let mut costs = Vec::new();
    for count in [1000, 10000] {
        let rows = (0..count / 20)
            .map(|row| {
                let children = (0..20)
                    .map(|column| {
                        Probe::new(
                            Kind::Group,
                            Rect::new(
                                column as f64 * 160.0,
                                0.0,
                                column as f64 * 160.0 + 100.0,
                                100.0,
                            ),
                            TRIO,
                            vec![],
                        ) as Rc<dyn InstanceNode>
                    })
                    .collect();
                Probe::new(
                    Kind::Scroll,
                    Rect::new(0.0, row as f64 * 200.0, 320.0, row as f64 * 200.0 + 100.0),
                    &[],
                    children,
                ) as Rc<dyn InstanceNode>
            })
            .collect();
        let outer = Probe::new(Kind::Scroll, Rect::new(0.0, 0.0, 320.0, 240.0), &[], rows);
        let (mut engine, root) = mount(vec![outer]);
        engine.tick();
        events();
        let outer = root.children.get()[0].clone();
        scroll(&engine.runtime_context, &outer, 0.0, 4000.0);
        engine.tick();
        events();
        let before = engine.runtime_context.viewport_proximity_stats();
        let rebuilds = engine.runtime_context.occlusion_stats().rebuilds;
        engine.runtime_context.update_scroller_surface_scroll(
            outer.id.to_u32(),
            0.0,
            4005.0,
            0.0,
            4005.0,
        );
        engine.tick();
        let after = engine.runtime_context.viewport_proximity_stats();
        assert_eq!(engine.runtime_context.occlusion_stats().rebuilds, rebuilds);
        assert_eq!(after.index_updates, before.index_updates);
        costs.push((
            after.domain_queries - before.domain_queries,
            after.candidates - before.candidates,
            after.samples - before.samples,
        ));
        events();
    }
    assert_eq!(costs[0], costs[1]);
    assert!(costs[1].2 < 60, "{costs:?}");
}

#[test]
fn mounted_reparent_keeps_instance_history_and_rebinds_scroll_domains() {
    let a = Probe::new(
        Kind::Scroll,
        Rect::new(0.0, 0.0, 320.0, 240.0),
        &[],
        vec![Probe::new(Kind::Group, rect(0.0), TRIO, vec![])],
    );
    let b = Probe::new(
        Kind::Scroll,
        Rect::new(0.0, 600.0, 320.0, 840.0),
        &[],
        vec![],
    );
    let (mut engine, root) = mount(vec![a, b]);
    engine.tick();
    let initial = events();
    let a = root.children.get()[0].clone();
    let b = root.children.get()[1].clone();
    let target = a.children.get()[0].clone();
    let children = b.attach_children(
        vec![target.clone()],
        &engine.runtime_context,
        &LocalProperty::new(Some(b.id)),
    );
    b.children.set(children);
    engine.tick();
    let moved = events();
    assert_eq!(kinds(&moved), ["change", "exit"]);
    assert!(moved.iter().all(|event| event.id == initial[0].id));
    assert_eq!(moved[0].previous, Some(initial[0].current));
    y(&b, 0.0);
    engine.tick();
    let returned = events();
    assert_eq!(kinds(&returned), ["enter", "change"]);
    assert!(returned[1].previous.is_none());
}

#[test]
fn rotated_collapsed_layout_has_no_area() {
    let (mut engine, root) = mount(vec![Probe::new(Kind::Group, rect(0.0), TRIO, vec![])]);
    let node = root.children.get()[0].clone();
    node.get_common_properties()
        .borrow()
        .width
        .set(Some(Size::Pixels(0.0.into())));
    node.get_common_properties()
        .borrow()
        .rotate
        .set(Some(Rotation::Degrees(45.0.into())));
    engine.tick();
    assert!(events().is_empty());
}

#[path = "support/counting_allocator.rs"]
mod counting_allocator;

/// Run in a fresh process for each PAX_BENCH_N / PAX_BENCH_OBSERVE combination.
/// This measures mounted runtime work and handler delivery, excluding GPU rendering.
#[test]
#[ignore = "manual timing/allocation measurement"]
fn observation_workload_measurement() {
    use counting_allocator::{ALLOCATIONS, LIVE_BYTES};
    use std::sync::atomic::Ordering::Relaxed;
    use std::time::Instant;
    let count: usize = std::env::var("PAX_BENCH_N").unwrap().parse().unwrap();
    let observe = std::env::var("PAX_BENCH_OBSERVE").unwrap() == "1";
    let baseline = LIVE_BYTES.load(Relaxed);
    let start = Instant::now();
    let children = (0..count)
        .map(|i| {
            Probe::new(
                Kind::Paint,
                rect(i as f64 * 200.0),
                if observe { TRIO } else { &[] },
                vec![],
            ) as Rc<dyn InstanceNode>
        })
        .collect();
    let outer = Probe::new(
        Kind::Scroll,
        Rect::new(0.0, 0.0, 320.0, 240.0),
        &[],
        children,
    );
    let (mut engine, root) = mount(vec![outer]);
    engine.tick();
    events();
    let outer = root.children.get()[0].clone();
    scroll(&engine.runtime_context, &outer, 0.0, 4000.0);
    engine.tick();
    events();
    let initial_ms = start.elapsed().as_secs_f64() * 1000.0;
    let bytes = LIVE_BYTES.load(Relaxed) - baseline;
    let allocations = ALLOCATIONS.load(Relaxed);
    let stats = engine.runtime_context.viewport_proximity_stats();
    let start = Instant::now();
    for i in 0..200 {
        let offset = 4000.0 + i as f64 * 5.0;
        engine.runtime_context.update_scroller_surface_scroll(
            outer.id.to_u32(),
            0.0,
            offset,
            0.0,
            offset,
        );
        engine.tick();
        events();
    }
    let us = start.elapsed().as_secs_f64() * 1e6 / 200.0;
    let after = engine.runtime_context.viewport_proximity_stats();
    println!("nodes={count} observe={observe} initial_ms={initial_ms:.2} live_bytes={bytes} scroll_us/tick={us:.2} allocations/tick={:.2} samples/tick={:.2} index_updates={}",
        (ALLOCATIONS.load(Relaxed)-allocations) as f64/200.0,(after.samples-stats.samples) as f64/200.0,after.index_updates-stats.index_updates);
}

#[test]
fn replacement_updates_bindings_without_leaking_instance_history() {
    let (mut engine, root) = mount(vec![
        Probe::new(Kind::Group, rect(0.0), TRIO, vec![]),
        Probe::new(Kind::Group, rect(0.0), TRIO, vec![]),
    ]);
    engine.tick();
    let initial = events();
    assert_eq!(initial.len(), 4);
    let a = root.children.get()[0].clone();
    let b = root.children.get()[1].clone();
    a.recreate_with_new_data(
        Probe::new(Kind::Group, rect(0.0), &[], vec![]),
        &engine.runtime_context,
    );
    engine.tick();
    assert!(events().is_empty());
    y(&b, 30.0);
    engine.tick();
    let moved = events();
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].id, b.id.to_u32());
    assert!(moved[0].previous.is_some());
    a.recreate_with_new_data(
        Probe::new(Kind::Group, rect(0.0), TRIO, vec![]),
        &engine.runtime_context,
    );
    engine.tick();
    let added = events();
    assert_eq!(kinds(&added), ["enter", "change"]);
    assert_eq!(added[0].id, a.id.to_u32());
    assert!(added[1].previous.is_none());
}

// The complete reconciliation is the oracle: it must not correct any mask
// emitted by the indexed path, including empty masks after leaving old coverage.
fn assert_masks_match_full(engine: &mut PaxEngine, native: &[Rc<ExpandedNode>]) {
    let hashes: Vec<_> = native.iter().map(|n| n.native_mask_hash.get()).collect();
    engine.runtime_context.mark_occlusion_dirty();
    engine.tick();
    assert_eq!(
        hashes,
        native
            .iter()
            .map(|n| n.native_mask_hash.get())
            .collect::<Vec<_>>()
    );
}

#[test]
fn indexed_native_masks_match_full_for_movement_opacity_clipping_and_structure() {
    TRACK_OCCLUSION.with(|flag| flag.set(true));
    let paint = Probe::new(Kind::Paint, rect(0.0), &[], vec![]);
    let clip = Probe::new(Kind::Clip, rect(0.0), &[], vec![paint]);
    let (mut engine, root) = mount(vec![clip, Probe::new(Kind::Native, rect(0.0), &[], vec![])]);
    engine.tick();
    let clip = root.children.get()[0].clone();
    let paint = clip.children.get()[0].clone();
    let native = root.children.get()[1].clone();
    assert_ne!(native.native_mask_hash.get(), 0);
    for (offset, opacity) in [
        (30.0, 1.0),
        (150.0, 1.0),
        (0.0, 0.0),
        (0.0, 0.5),
        (-20.0, 1.0),
    ] {
        y(&paint, offset);
        paint
            .get_common_properties()
            .borrow()
            .opacity
            .set(Some(opacity.into()));
        let before = engine.runtime_context.occlusion_stats().rebuilds;
        engine.tick();
        assert_eq!(engine.runtime_context.occlusion_stats().rebuilds, before);
        if offset == 150.0 || opacity == 0.0 {
            assert_eq!(native.native_mask_hash.get(), 0);
        }
        assert_masks_match_full(&mut engine, &[native.clone()]);
    }
    y(&native, 60.0);
    native
        .get_common_properties()
        .borrow()
        .rotate
        .set(Some(Rotation::Degrees(15.0.into())));
    engine.tick();
    assert_masks_match_full(&mut engine, &[native.clone()]);
    y(&clip, 50.0);
    engine.tick();
    assert_masks_match_full(&mut engine, &[native.clone()]);
    paint
        .get_common_properties()
        .borrow()
        .unclippable
        .set(Some(true));
    y(&paint, -50.0);
    engine.tick();
    assert_masks_match_full(&mut engine, &[native.clone()]);
    // Order changes and deletion take the structural path and remove old coverage.
    root.children.set(vec![native.clone(), clip.clone()]);
    engine.tick();
    assert_eq!(native.native_mask_hash.get(), 0);
    clip.children.set(vec![]);
    engine.tick();
    assert_masks_match_full(&mut engine, &[native]);
    TRACK_OCCLUSION.with(|flag| flag.set(false));
}

#[test]
fn indexed_native_masks_and_scroll_queries_stay_local_in_large_scene() {
    TRACK_OCCLUSION.with(|flag| flag.set(true));
    let mut children: Vec<Rc<dyn InstanceNode>> = Vec::new();
    for i in 0..1000 {
        children.push(Probe::new(Kind::Paint, rect(i as f64 * 200.0), &[], vec![]));
        children.push(Probe::new(
            Kind::Native,
            rect(i as f64 * 200.0),
            &[],
            vec![],
        ));
    }
    let (mut engine, root) = mount(vec![Probe::new(
        Kind::Scroll,
        Rect::new(0.0, 0.0, 320.0, 240.0),
        &[],
        children,
    )]);
    engine.tick();
    let scroller = root.children.get()[0].clone();
    scroll(&engine.runtime_context, &scroller, 0.0, 0.0);
    engine.tick();
    let paint = scroller.children.get()[0].clone();
    let natives: Vec<_> = scroller
        .children
        .get()
        .iter()
        .skip(1)
        .step_by(2)
        .cloned()
        .collect();
    let before = engine.runtime_context.occlusion_stats();
    y(&paint, 10.0);
    engine.tick();
    let after = engine.runtime_context.occlusion_stats();
    assert_eq!(after.rebuilds, before.rebuilds);
    assert!(after.records_updated - before.records_updated <= 2);
    assert!(after.masks_evaluated - before.masks_evaluated <= 2);
    assert!(after.mask_candidates - before.mask_candidates <= 3);
    assert_masks_match_full(&mut engine, &natives);
    for offset in [50.0, 250.0, 2000.0, 1950.0, 0.0] {
        let before = engine.runtime_context.occlusion_stats();
        let geometry_before = engine.runtime_context.scene_geometry_stats();
        scroll(&engine.runtime_context, &scroller, 0.0, offset);
        engine.tick();
        let after = engine.runtime_context.occlusion_stats();
        assert_eq!(after.rebuilds, before.rebuilds);
        assert!(after.masks_evaluated - before.masks_evaluated <= 6);
        assert_eq!(
            engine.runtime_context.scene_geometry_stats().index_updates,
            geometry_before.index_updates
        );
        assert_masks_match_full(&mut engine, &natives);
    }
    TRACK_OCCLUSION.with(|flag| flag.set(false));
}

#[test]
fn nested_scroll_masks_remain_equivalent_to_full_reconciliation() {
    TRACK_OCCLUSION.with(|flag| flag.set(true));
    let inner = Probe::new(
        Kind::Scroll,
        rect(50.0),
        &[],
        vec![
            Probe::new(Kind::Paint, rect(0.0), &[], vec![]),
            Probe::new(Kind::Native, rect(0.0), &[], vec![]),
        ],
    );
    let (mut engine, root) = mount(vec![Probe::new(
        Kind::Scroll,
        Rect::new(0.0, 0.0, 320.0, 240.0),
        &[],
        vec![inner],
    )]);
    engine.tick();
    let outer = root.children.get()[0].clone();
    let inner = outer.children.get()[0].clone();
    let paint = inner.children.get()[0].clone();
    let native = inner.children.get()[1].clone();
    for offset in [0.0, 20.0, 100.0, 0.0] {
        scroll(&engine.runtime_context, &outer, 0.0, offset);
        scroll(&engine.runtime_context, &inner, 0.0, offset / 2.0);
        y(&paint, offset / 3.0);
        engine.tick();
        assert_masks_match_full(&mut engine, &[native.clone()]);
    }
    TRACK_OCCLUSION.with(|flag| flag.set(false));
}

#[test]
fn masks_with_scroll_islands_disabled_use_presented_geometry() {
    TRACK_OCCLUSION.with(|flag| flag.set(true));
    let (mut engine, root) = mount(vec![
        Probe::new(Kind::Paint, rect(0.0), &[], vec![]),
        Probe::new(
            Kind::Scroll,
            Rect::new(0.0, 0.0, 320.0, 240.0),
            &[],
            vec![Probe::new(Kind::Native, rect(1000.0), &[], vec![])],
        ),
    ]);
    engine
        .runtime_context
        .globals()
        .browser_allows_scroller_vector_layers
        .set(false);
    engine.tick();
    let paint = root.children.get()[0].clone();
    let scroller = root.children.get()[1].clone();
    let native = scroller.children.get()[0].clone();
    scroll(&engine.runtime_context, &scroller, 0.0, 1000.0);
    engine.tick();
    assert_ne!(native.native_mask_hash.get(), 0);
    y(&paint, 300.0);
    engine.tick();
    assert_eq!(native.native_mask_hash.get(), 0);
    assert_masks_match_full(&mut engine, &[native]);
    TRACK_OCCLUSION.with(|flag| flag.set(false));
}

#[test]
fn unclipped_scroll_overflow_updates_masks_outside_its_viewport() {
    TRACK_OCCLUSION.with(|flag| flag.set(true));
    let (mut engine, root) = mount(vec![Probe::new(
        Kind::Clip,
        Rect::new(0.0, 0.0, 500.0, 500.0),
        &[],
        vec![Probe::new(
            Kind::UnclippedScroll,
            rect(0.0),
            &[],
            vec![
                Probe::new(Kind::Paint, rect(300.0), &[], vec![]),
                Probe::new(Kind::Native, rect(300.0), &[], vec![]),
            ],
        )],
    )]);
    engine.tick();
    let scroller = root.children.get()[0].children.get()[0].clone();
    let native = scroller.children.get()[1].clone();
    scroll(&engine.runtime_context, &scroller, 0.0, 0.0);
    engine.tick();
    assert_ne!(native.native_mask_hash.get(), 0);
    scroll(&engine.runtime_context, &scroller, 0.0, 450.0);
    engine.tick();
    assert_eq!(native.native_mask_hash.get(), 0);
    assert_masks_match_full(&mut engine, &[native]);
    TRACK_OCCLUSION.with(|flag| flag.set(false));
}

#[test]
fn detached_mask_source_does_not_register_viewport_events() {
    let (mut engine, root) = mount(Vec::new());
    let context = engine.runtime_context.clone();
    let source = Probe::new(
        Kind::Paint,
        rect(0.0),
        &[
            "viewport_proximity_enter",
            "viewport_proximity_change",
            "viewport_proximity_exit",
        ],
        vec![],
    );
    let source = root
        .generate_children(
            vec![(source as Rc<dyn InstanceNode>, root.stack.clone())],
            &context,
            &root.parent_frame,
            false,
        )
        .remove(0);
    source.mount_as_render_source(&root, &context);
    engine.tick();
    assert!(events().is_empty());
    assert_eq!(context.viewport_proximity_stats(), Default::default());
    source
        .get_common_properties()
        .borrow()
        .y
        .set(Some(Size::Pixels(800.0.into())));
    engine.tick();
    assert!(events().is_empty());
    assert_eq!(context.viewport_proximity_stats(), Default::default());
    source.recurse_unmount(&context);
}
