use super::*;
use crate::{
    scene_geometry::CanvasGeometry, BaseInstance, CommonPropertiesInit, ComponentInstance,
    InstanceFlags, InstanceNode, InstantiationArgs, PaxEngine, PropertiesInit, PropertiesScopeInit,
};
use kurbo::{Rect, Shape};
use pax_runtime_api::{pax_value::PaxAny, Platform, Rotation, Size, OS};
use std::cell::{Cell, RefCell};

fn args(children: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Default,
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, node| {
            node.is_none()
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
        })),
        handler_registry: None,
        children: Some(RefCell::new(children)),
        component_template: None,
        component_settings: None,
        component_camera: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
        properties_scope: PropertiesScopeInit::None,
    }
}

struct Surface {
    base: BaseInstance,
    content: LocalProperty<f64>,
    effects: Rc<Cell<usize>>,
}
impl InstanceNode for Surface {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    layer: Layer::Canvas,
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    is_component: false,
                    is_slot: false,
                },
            ),
            content: LocalProperty::new(0.),
            effects: Default::default(),
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
        f.debug_struct("Surface").finish()
    }
    fn supports_retained_motion(&self) -> bool {
        true
    }
    fn canvas_geometry_is_local(&self) -> bool {
        true
    }
    fn accepts_transferred_children(&self) -> bool {
        true
    }
    fn property_requires_occlusion_recompute(&self, _: &str) -> bool {
        false
    }
    fn property_requires_input_recompute(&self, _: &str) -> bool {
        false
    }
    fn handle_mount(self: Rc<Self>, node: &Rc<ExpandedNode>, ctx: &Rc<RuntimeContext>) {
        let children = self.base().get_instance_children().borrow().clone();
        let children = node.generate_children(
            children
                .into_iter()
                .zip(std::iter::repeat(node.stack.clone())),
            ctx,
            &node.parent_frame,
            true,
        );
        node.children.set(children);
        let mut deps = node.canvas_placement_dependencies();
        deps.push(self.content.untyped());
        let content = self.content.clone();
        let effects = self.effects.clone();
        let weak = Rc::downgrade(node);
        let context = Rc::downgrade(ctx);
        node.changed_listener.replace_with(LocalProperty::computed(
            move || {
                content.get();
                effects.set(effects.get() + 1);
                if let (Some(node), Some(ctx)) = (weak.upgrade(), context.upgrade()) {
                    ctx.mark_canvas_node_dirty(node.id);
                    ctx.set_canvas_dirty(node.occlusion.get().render_layer_id);
                }
            },
            &deps,
        ));
    }
    fn prepare_canvas_geometry(&self, node: &ExpandedNode) -> CanvasGeometry {
        let (w, h) = node.layout_bounds.get();
        let path = Rect::new(0., 0., w + self.content.get(), h).to_path(0.1);
        CanvasGeometry {
            local_bounds: Some(path.bounding_box()),
            path: Some(Rc::new(path)),
        }
    }
}

fn fixture(
    enabled: bool,
    count: usize,
) -> (
    PaxEngine,
    Rc<ExpandedNode>,
    Vec<Rc<ExpandedNode>>,
    Rc<Surface>,
) {
    let surface = Surface::instantiate(args(vec![]));
    let parent = Surface::instantiate(args(
        (0..count)
            .map(|_| surface.clone() as Rc<dyn InstanceNode>)
            .collect(),
    ));
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![parent]));
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    engine.runtime_context.retained_motion.borrow_mut().enabled = enabled;
    engine
        .runtime_context
        .edit_globals(|g| g.renderer_is_gpu = true);
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.runtime_context.resize_canvas_layers_to(1);
    engine.tick();
    let parent = root.children.get()[0].clone();
    let children = parent.children.get();
    for (i, node) in children.iter().enumerate() {
        let cp = node.get_common_properties().borrow().clone();
        cp.width.set(Some(Size::Pixels(20.0.into())));
        cp.height.set(Some(Size::Pixels(20.0.into())));
        cp.x.set(Some(Size::Pixels(((i % 20) as f64 * 25.).into())));
        cp.y.set(Some(Size::Pixels(((i / 20) as f64 * 25.).into())));
    }
    engine.tick();
    (engine, parent, children, surface)
}

#[test]
fn inherited_motion_preserves_content_but_prepares_current_world_records() {
    for enabled in [false, true] {
        let (mut engine, parent, children, surface) = fixture(enabled, 32);
        let ctx = engine.runtime_context.clone();
        let before: Vec<_> = children
            .iter()
            .map(|n| ctx.canvas_geometry_for_node(n))
            .collect();
        let effects = surface.effects.get();
        let preparations = ctx.scene_geometry_stats().local_preparations;
        let notifications = ctx.retained_motion.borrow().notifications;
        for angle in [20., -20., 89., 90., 91., 180.] {
            parent
                .get_common_properties()
                .borrow()
                .rotate_y
                .set(Some(Rotation::Degrees(angle.into())));
            engine.tick();
            for (node, original) in children.iter().zip(&before) {
                let current = ctx.canvas_geometry_for_node(node);
                assert_eq!(current.spatial_geometry, node.presentation_geometry.get());
                if enabled {
                    assert!(Rc::ptr_eq(
                        current.local.path.as_ref().unwrap(),
                        original.local.path.as_ref().unwrap()
                    ));
                }
            }
        }
        if enabled {
            assert_eq!(
                surface.effects.get(),
                effects,
                "ancestor rotation must not run child content effects"
            );
            assert_eq!(ctx.scene_geometry_stats().local_preparations, preparations);
            assert_eq!(
                ctx.retained_motion.borrow().notifications - notifications,
                6
            );
        } else {
            assert!(surface.effects.get() >= effects + 32 * 6);
            assert!(ctx.scene_geometry_stats().local_preparations >= preparations + 32 * 6);
        }
        surface.content.set(3.);
        engine.tick();
        assert!(surface.effects.get() > effects);
        assert_eq!(
            ctx.canvas_geometry_for_node(&children[0])
                .local
                .local_bounds
                .unwrap()
                .width(),
            23.
        );
        children[0]
            .get_common_properties()
            .borrow()
            .width
            .set(Some(Size::Pixels(40.0.into())));
        engine.tick();
        assert_eq!(
            ctx.canvas_geometry_for_node(&children[0])
                .local
                .local_bounds
                .unwrap()
                .width(),
            43.
        );
    }
}

#[test]
fn nested_changes_coalesce_and_pending_input_uses_current_placement() {
    let (mut engine, parent, children, _) = fixture(true, 4);
    let ctx = engine.runtime_context.clone();
    let query = |point: (f64, f64)| {
        ctx.get_elements_beneath_ray(
            None,
            pax_runtime_api::math::Point2::new(point.0, point.1),
            false,
            vec![],
            false,
        )
        .into_iter()
        .map(|n| n.id)
        .collect::<Vec<_>>()
    };
    assert!(query((5., 5.)).contains(&children[0].id));
    let visited = ctx.retained_motion.borrow().visited;
    parent
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(100.0.into())));
    children[0]
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(10.0.into())));
    ctx.drain_node_effects();
    let pending = ctx.input_query_stats().pending_queries;
    assert!(query((115., 5.)).contains(&children[0].id));
    assert!(!query((5., 5.)).contains(&children[0].id));
    assert!(ctx.input_query_stats().pending_queries > pending);
    engine.tick();
    assert_eq!(ctx.retained_motion.borrow().visited - visited, 5);
    assert!(query((115., 5.)).contains(&children[0].id));
    assert!(!query((5., 5.)).contains(&children[0].id));
}

#[test]
fn camera_and_move_progress_invalidate_placement_without_rebuilding_content() {
    let (mut engine, parent, children, surface) = fixture(true, 2);
    let ctx = engine.runtime_context.clone();
    let effects = surface.effects.get();
    parent
        .own_camera_view
        .set(Some(crate::camera::resolve_for_test(
            parent.spatial_geometry.get(),
            [310., 230., 0.],
            [0.; 3],
            1.1,
        )));
    engine.tick();
    assert_eq!(
        ctx.canvas_geometry_for_node(&children[0]).spatial_geometry,
        children[0].presentation_geometry.get()
    );
    assert_eq!(surface.effects.get(), effects);
    parent.own_camera_view.set(None);
    engine.tick();
    parent
        .move_origin
        .set(Some(crate::presentation::MoveOrigin {
            transform: pax_runtime_api::math::Transform2::translate(
                pax_runtime_api::math::Vector2::new(100., 0.),
            ),
            bounds: parent.layout_bounds.get(),
        }));
    parent.move_progress.set(0.25);
    engine.tick();
    let before = ctx.canvas_geometry_for_node(&children[0]).world_transform;
    parent.move_progress.set(0.75);
    engine.tick();
    assert_ne!(
        ctx.canvas_geometry_for_node(&children[0]).world_transform,
        before
    );
    assert_eq!(
        ctx.canvas_geometry_for_node(&children[0]).spatial_geometry,
        children[0].presentation_geometry.get()
    );
}

#[test]
fn a_detached_flight_tracks_its_moving_destination_at_fixed_progress() {
    let (mut engine, parent, children, _) = fixture(true, 2);
    let ctx = engine.runtime_context.clone();
    let flight = children[0].clone();
    flight
        .move_origin
        .set(Some(crate::presentation::MoveOrigin {
            transform: pax_runtime_api::math::Transform2::identity(),
            bounds: flight.layout_bounds.get(),
        }));
    flight.move_progress.set(0.5);
    flight.move_in_flight.set(true);
    flight.rebind_move_presentation(&ctx);
    engine.tick();
    let before = ctx.canvas_geometry_for_node(&flight).world_transform;
    let notifications = ctx.retained_motion.borrow().notifications;
    parent
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(100.0.into())));
    engine.tick();
    // Both the destination and the separately presented flight must notify.
    assert!(ctx.retained_motion.borrow().notifications - notifications >= 2);
    assert_ne!(
        ctx.canvas_geometry_for_node(&flight).world_transform,
        before
    );
    assert_eq!(
        ctx.canvas_geometry_for_node(&flight).spatial_geometry,
        flight.presentation_geometry.get()
    );
}

#[test]
fn reparenting_and_viewport_motion_keep_placement_current() {
    let (mut engine, parent, children, _) = fixture(true, 3);
    let ctx = engine.runtime_context.clone();
    let moved = children[0].clone();
    let destination = children[1].clone();
    crate::NodeRef::new(&moved, &ctx)
        .move_to_append_child(&crate::NodeRef::new(&destination, &ctx))
        .unwrap();
    engine.tick();
    destination
        .get_common_properties()
        .borrow()
        .rotate_y
        .set(Some(Rotation::Degrees(30.0.into())));
    engine.tick();
    assert_eq!(
        ctx.canvas_geometry_for_node(&moved).spatial_geometry,
        moved.presentation_geometry.get()
    );
    let before = ctx.canvas_geometry_for_node(&moved).world_transform;
    let viewport = ctx.globals().viewport;
    let mut value = viewport.get();
    value.transform =
        pax_runtime_api::math::Transform2::translate(pax_runtime_api::math::Vector2::new(50., 20.));
    viewport.set(value);
    engine.tick();
    assert_ne!(ctx.canvas_geometry_for_node(&moved).world_transform, before);
    assert_eq!(
        ctx.canvas_geometry_for_node(&moved).spatial_geometry,
        moved.presentation_geometry.get()
    );
    parent
        .get_common_properties()
        .borrow()
        .rotate_y
        .set(Some(Rotation::Degrees(15.0.into())));
    ctx.drain_node_effects();
    moved.clone().recurse_unmount(&ctx);
    engine.tick();
    assert!(!ctx.retained_motion.borrow().dirty.contains(&moved.id));
}

#[test]
#[ignore = "manual CPU-only A/B timing; run alone in release with --nocapture --test-threads=1"]
fn motion_tick_bench() {
    for count in [32, 1024] {
        for enabled in [false, true] {
            let (mut engine, parent, _, surface) = fixture(enabled, count);
            let rotation = parent.get_common_properties().borrow().rotate_y.clone();
            for i in 0..20 {
                rotation.set(Some(Rotation::Degrees((i as f64).into())));
                engine.tick();
            }
            let effects = surface.effects.get();
            let preparations = engine
                .runtime_context
                .scene_geometry_stats()
                .local_preparations;
            let start = std::time::Instant::now();
            let frames = 120;
            for i in 0..frames {
                rotation.set(Some(Rotation::Degrees((i as f64 * 0.3 - 20.).into())));
                engine.tick();
            }
            println!("children={count} retained={enabled} tick_ms={:.3} child_content_effects={} local_preparations={}",
                start.elapsed().as_secs_f64()*1000./frames as f64,
                surface.effects.get()-effects,
                engine.runtime_context.scene_geometry_stats().local_preparations-preparations);
        }
    }
}
