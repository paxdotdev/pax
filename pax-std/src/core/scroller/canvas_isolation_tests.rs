use super::*;
use crate::core::frame::{Frame, FrameInstance};
use crate::drawing::rectangle::{Rectangle, RectangleInstance};
use pax_runtime::api::{pax_value::ToFromPaxAny, CommonProperties, Platform, Rotation, OS};
use pax_runtime::{
    CommonPropertiesInit, ComponentInstance, PaxEngine, PropertiesInit, PropertiesScopeInit,
};

fn args<T: ToFromPaxAny + Clone + 'static>(
    props: T,
    children: Vec<Rc<dyn InstanceNode>>,
) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(CommonProperties::default())))
        })),
        prototypical_properties: PropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(props.clone().to_pax_any())))
        })),
        properties_scope: PropertiesScopeInit::None,
        children: Some(RefCell::new(children)),
        component_template: None,
        component_settings: None,
        component_camera: None,
        handler_registry: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
    }
}

struct Fixture {
    engine: PaxEngine,
    outer: Rc<ExpandedNode>,
    scroller: Rc<ExpandedNode>,
    inner: Rc<ExpandedNode>,
    cards: Vec<Rc<ExpandedNode>>,
}

impl Fixture {
    fn new(count: usize, nested_layers: bool) -> Self {
        Self::with_wrapper(count, nested_layers, false)
    }

    fn with_wrapper(count: usize, nested_layers: bool, wrap: bool) -> Self {
        let cards = (0..count)
            .map(|_| {
                RectangleInstance::instantiate(args(Rectangle::default(), vec![]))
                    as Rc<dyn InstanceNode>
            })
            .collect();
        let inner = FrameInstance::instantiate(args(Frame::default(), cards));
        let content: Rc<dyn InstanceNode> = if wrap {
            let mut wrapper = args(Frame::default(), vec![]);
            wrapper.component_template = Some(RefCell::new(vec![inner]));
            ComponentInstance::instantiate(wrapper)
        } else {
            inner
        };
        let scroller =
            ScrollerHostInstance::instantiate(args(ScrollerHost::default(), vec![content]));
        let outer = FrameInstance::instantiate(args(Frame::default(), vec![scroller]));
        let outer_scroll =
            ScrollerHostInstance::instantiate(args(ScrollerHost::default(), vec![outer]));
        let mut root_args = args(Frame::default(), vec![]);
        root_args.component_template = Some(RefCell::new(vec![outer_scroll]));
        let context = Rc::new(crate::test_support::runtime_context_with_target(
            (640., 480.),
            Platform::Web,
            OS::Mac,
        ));
        context
            .globals()
            .browser_allows_nested_scroller_vector_layers
            .set(nested_layers);
        let mut engine = PaxEngine {
            runtime_context: context,
            root_expanded_node: None,
            scroller_tiling_policy: Default::default(),
        };
        let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
        let outer_scroll = root.children.get()[0].clone();
        let outer = outer_scroll.children.get()[0].clone();
        let scroller = outer.children.get()[0].clone();
        let inner = scroller.children.get()[0].clone();
        let inner = if wrap {
            inner.children.get()[0].clone()
        } else {
            inner
        };
        let cards = inner.children.get();
        let mut fixture = Self {
            engine,
            outer,
            scroller,
            inner,
            cards,
        };
        fixture.settle();
        fixture.clear();
        fixture
    }

    fn settle(&mut self) {
        for _ in 0..3 {
            self.engine.tick();
        }
    }
    fn settle_messages(&mut self) -> Vec<pax_message::NativeMessage> {
        (0..3).flat_map(|_| self.engine.tick()).collect()
    }
    fn clear(&self) {
        let ctx = &self.engine.runtime_context;
        for id in ctx.dirty_canvas_node_ids() {
            ctx.clear_canvas_node_dirty(&id);
        }
        ctx.clear_all_dirty_canvases();
        ctx.take_native_messages();
    }
    fn any_dirty(&self) -> bool {
        self.cards
            .iter()
            .any(|n| self.engine.runtime_context.is_canvas_node_dirty(&n.id))
    }

    fn assert_local_matches_world(&self) {
        let owner = kurbo::Affine::from(
            self.scroller
                .presentation_transform_and_bounds
                .get()
                .transform,
        );
        for card in &self.cards {
            let world = kurbo::Affine::from(card.presentation_transform_and_bounds.get().transform);
            let expected = (owner.inverse() * world).as_coeffs();
            let actual = self
                .engine
                .runtime_context
                .canvas_geometry_for_node(card)
                .surface_transform
                .as_coeffs();
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| (a - b).abs() < 1e-9),
                "local composition must preserve placement: {actual:?} != {expected:?}"
            );
        }
    }
}

#[test]
fn moving_nested_scroller_retains_card_paint_and_surface_coverage() {
    for count in [1, 128] {
        let mut f = Fixture::new(count, true);
        assert!(f
            .cards
            .iter()
            .all(|n| n.has_surface_local_canvas_placement()));
        let before: Vec<_> = f
            .cards
            .iter()
            .map(|n| f.engine.runtime_context.canvas_geometry_for_node(n))
            .collect();
        for (x, angle) in [(20., 3.), (-12., -7.), (0., 0.)] {
            let geometry_before = f.engine.runtime_context.scene_geometry_stats();
            let occlusion_before = f.engine.runtime_context.occlusion_stats();
            let props = f.outer.get_common_properties();
            props.borrow().x.set(Some(Size::Pixels(x.into())));
            props
                .borrow()
                .rotate
                .set(Some(Rotation::Degrees(angle.into())));
            let messages = f.settle_messages();
            assert!(!messages.iter().any(|m| matches!(m,
                pax_message::NativeMessage::FrameUpdate(p)
                    if p.id == f.inner.id.to_u32() && (p.transform.is_some() || p.clip_path.is_some())
            )), "host movement must not patch a retained Frame");
            let geometry_after = f.engine.runtime_context.scene_geometry_stats();
            let occlusion_after = f.engine.runtime_context.occlusion_stats();
            assert!(
                geometry_after.preparations - geometry_before.preparations <= 8,
                "host-only geometry work grew with {count} cards: {}",
                geometry_after.preparations - geometry_before.preparations
            );
            assert!(
                occlusion_after.records_updated - occlusion_before.records_updated <= 8,
                "host-only coverage work grew with {count} cards: {}",
                occlusion_after.records_updated - occlusion_before.records_updated
            );
            assert!(
                !f.any_dirty(),
                "moving {count} cards must not re-record their paint"
            );
            f.assert_local_matches_world();
            for (node, old) in f.cards.iter().zip(&before) {
                let next = f.engine.runtime_context.canvas_geometry_for_node(node);
                assert_eq!(next.surface_transform, old.surface_transform);
                assert_eq!(next.coverage_bounds, old.coverage_bounds);
                assert_eq!(
                    next.world_transform,
                    kurbo::Affine::from(node.presentation_transform_and_bounds.get().transform)
                );
            }
        }
    }
}

#[test]
fn inner_placement_dimensions_and_paint_still_invalidate_cards() {
    let mut f = Fixture::new(4, true);
    f.inner
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(11.into())));
    f.settle();
    assert!(f.any_dirty());
    f.assert_local_matches_world();
    f.clear();
    f.cards[0]
        .get_common_properties()
        .borrow()
        .width
        .set(Some(Size::Pixels(93.into())));
    f.settle();
    assert!(f
        .engine
        .runtime_context
        .is_canvas_node_dirty(&f.cards[0].id));
    assert_eq!(
        f.engine
            .runtime_context
            .canvas_geometry_for_node(&f.cards[0])
            .bounds
            .0,
        93.
    );
    f.clear();
    f.cards[0].with_properties_unwrapped(|r: &mut Rectangle| {
        r.fill.set(vec![pax_runtime::api::Fill::from(
            pax_runtime::api::Color::RED,
        )])
    });
    f.settle();
    assert!(f
        .engine
        .runtime_context
        .is_canvas_node_dirty(&f.cards[0].id));
}

#[test]
fn moving_host_retains_input_bounds_but_uses_current_world_coordinates() {
    use pax_runtime::api::math::Point2;
    for count in [1, 128] {
        let mut f = Fixture::new(count, true);
        for card in &f.cards {
            let props = card.get_common_properties();
            props.borrow().width.set(Some(Size::Pixels(30.into())));
            props.borrow().height.set(Some(Size::Pixels(20.into())));
        }
        f.settle();
        let query = |f: &Fixture, x, y| {
            f.engine
                .runtime_context
                .get_elements_beneath_ray(None, Point2::new(x, y), false, vec![], false)
                .iter()
                .filter(|n| f.cards.iter().any(|c| c.id == n.id))
                .count()
        };
        assert_eq!(query(&f, 15., 10.), count);
        let before = f.engine.runtime_context.input_query_stats();
        f.outer
            .get_common_properties()
            .borrow()
            .x
            .set(Some(Size::Pixels(70.into())));
        f.settle();
        assert_eq!(query(&f, 85., 10.), count);
        assert_eq!(query(&f, 15., 10.), 0);
        let after = f.engine.runtime_context.input_query_stats();
        assert!(
            after.bounds_prepared - before.bounds_prepared <= 4,
            "host movement prepared {} hit bounds for {count} children",
            after.bounds_prepared - before.bounds_prepared
        );
        f.cards[0]
            .get_common_properties()
            .borrow()
            .x
            .set(Some(Size::Pixels(50.into())));
        f.settle();
        assert_eq!(query(&f, 85., 10.), count - 1);
        assert_eq!(query(&f, 135., 10.), 1);
    }
}

#[test]
fn shared_layer_nested_scroller_motion_still_invalidates_its_paint() {
    let mut f = Fixture::new(3, false);
    f.scroller
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(17.into())));
    f.settle();
    assert!(
        f.any_dirty(),
        "no independent surface means placement changes inside the outer surface"
    );
}

#[test]
fn disabled_frame_resizes_without_repainting_fixed_children() {
    let mut f = Fixture::new(1, true);
    let properties = f.cards[0].get_common_properties();
    properties.borrow().width.set(Some(Size::Pixels(32.into())));
    properties
        .borrow()
        .height
        .set(Some(Size::Pixels(32.into())));
    let width = f.inner.get_common_properties().borrow().width.clone();
    width.set(Some(Size::Pixels(98.into())));
    f.inner
        .with_properties_unwrapped(|frame: &mut Frame| frame._clip_content.set(false));
    f.settle();
    f.clear();

    width.set(Some(Size::Pixels(97.into())));
    let messages = f.settle_messages();
    assert!(messages.iter().any(|m| matches!(m,
        pax_message::NativeMessage::FrameUpdate(p)
            if p.id == f.inner.id.to_u32() && p.size_x == Some(97.)
    )));
    assert!(!f.any_dirty());

    for clips in [true, false] {
        f.inner
            .with_properties_unwrapped(|frame: &mut Frame| frame._clip_content.set(clips));
        f.settle();
        assert!(
            f.any_dirty(),
            "changing the aperture must repaint its contents"
        );
        f.clear();
    }
}

#[test]
fn resizing_an_outer_scrollport_preserves_fixed_shelf_drawings() {
    let mut f = Fixture::new(8, true);
    let root_scroll = f.engine.root_expanded_node.as_ref().unwrap().children.get()[0].clone();
    let width = root_scroll.get_common_properties().borrow().width.clone();
    width.set(Some(Size::Pixels(320.into())));
    f.outer
        .get_common_properties()
        .borrow()
        .width
        .set(Some(Size::Pixels(300.into())));
    let shelf = f.scroller.get_common_properties();
    shelf.borrow().width.set(Some(Size::Pixels(210.into())));
    for card in &f.cards {
        let properties = card.get_common_properties();
        properties.borrow().width.set(Some(Size::Pixels(32.into())));
        properties
            .borrow()
            .height
            .set(Some(Size::Pixels(24.into())));
    }
    f.settle();
    let ctx = f.engine.runtime_context.clone();
    let layer = f.cards[0].occlusion.get().render_layer_id;
    assert_eq!(ctx.get_layer_scroller_owner(layer), Some(f.scroller.id));
    let old: Vec<_> = f
        .cards
        .iter()
        .map(|node| ctx.canvas_geometry_for_node(node))
        .collect();
    f.clear();

    width.set(Some(Size::Pixels(319.into())));
    let messages = f.settle_messages();
    assert!(messages.iter().any(
        |m| matches!(m, pax_message::NativeMessage::ScrollerUpdate(p)
        if p.id == root_scroll.id.to_u32() && p.size_x == Some(319.))
    ));
    for (card, old) in f.cards.iter().zip(old) {
        let new = ctx.canvas_geometry_for_node(card);
        assert_eq!(new.bounds, old.bounds);
        assert_eq!(new.surface_transform, old.surface_transform);
        assert_eq!(new.local_paint_revision, old.local_paint_revision);
    }
    assert!(!f.any_dirty());
}

#[test]
fn resizing_a_shared_layer_clips_its_drawings_again() {
    let mut f = Fixture::new(1, false);
    let ctx = f.engine.runtime_context.clone();
    let card = f.cards[0].clone();
    assert_ne!(
        ctx.get_layer_scroller_owner(card.occlusion.get().render_layer_id),
        Some(f.scroller.id)
    );
    let width = f.scroller.get_common_properties().borrow().width.clone();
    width.set(Some(Size::Pixels(220.into())));
    card.get_common_properties()
        .borrow()
        .width
        .set(Some(Size::Pixels(256.into())));
    f.settle();
    let before = ctx.canvas_geometry_for_node(&card);
    width.set(Some(Size::Pixels(215.into())));
    f.settle();
    assert_ne!(
        ctx.canvas_geometry_for_node(&card).local_paint_revision,
        before.local_paint_revision
    );
}

#[test]
fn resizing_an_owned_surface_updates_percentage_children() {
    let mut f = Fixture::new(1, true);
    let ctx = f.engine.runtime_context.clone();
    let card = f.cards[0].clone();
    let width = f.scroller.get_common_properties().borrow().width.clone();
    card.get_common_properties()
        .borrow()
        .width
        .set(Some(Size::Percent(50.into())));
    width.set(Some(Size::Pixels(200.into())));
    f.settle();
    let before = ctx.canvas_geometry_for_node(&card);
    width.set(Some(Size::Pixels(210.into())));
    f.settle();
    let after = ctx.canvas_geometry_for_node(&card);
    assert_eq!((before.bounds.0, after.bounds.0), (100., 105.));
    assert_ne!(before.local_paint_revision, after.local_paint_revision);
}

#[test]
fn a_wrapped_frame_tracks_its_clip_in_the_owning_shelf() {
    let mut f = Fixture::with_wrapper(1, true, true);
    let ctx = f.engine.runtime_context.clone();
    let card = f.cards[0].clone();
    // Observe invalidation from the Frame alone. A drawable can independently
    // subscribe to world placement, as Rectangle does without retained motion.
    card.changed_listener.replace_with(LocalProperty::new(()));
    let frame = f.inner.get_common_properties();
    frame.borrow().x.set(Some(Size::Pixels(13.37.into())));
    frame.borrow().width.set(Some(Size::Pixels(91.into())));
    f.inner
        .with_properties_unwrapped(|p: &mut Frame| p.corner_radius.set(6.));
    let stage = f.outer.get_common_properties();
    f.settle();
    let before = ctx.canvas_geometry_for_node(&card).local_paint_revision;
    for scale in [76.3, 76.1, 76.3] {
        stage
            .borrow()
            .scale_x
            .set(Some(Size::Percent(scale.into())));
        f.settle();
        assert_eq!(
            ctx.canvas_geometry_for_node(&card).local_paint_revision,
            before
        );
    }

    // A real change inside the shelf still moves the clip and repaints the card.
    frame.borrow().x.set(Some(Size::Pixels(15.37.into())));
    let placement = f.inner.canvas_transform_and_bounds.get();
    assert!((Affine::from(placement.transform).as_coeffs()[4] - 15.37).abs() < 1e-9);
    f.settle();
    assert_ne!(
        ctx.canvas_geometry_for_node(&card).local_paint_revision,
        before
    );
}

#[test]
fn frame_observes_canvas_fallback_only_while_its_shelf_lacks_a_surface() {
    let mut f = Fixture::with_wrapper(1, true, true);
    let ctx = f.engine.runtime_context.clone();
    let fallback = f.inner.canvas_transform_and_bounds.untyped();
    let listener = f.inner.changed_listener.untyped();
    let observes_fallback =
        || pax_runtime::api::properties::property_has_direct_outbound(&fallback, &listener);
    for owns_surface in [true, false, true] {
        ctx.globals()
            .browser_allows_nested_scroller_vector_layers
            .set(owns_surface);
        ctx.mark_occlusion_dirty();
        f.engine.tick();
        assert_eq!(
            ctx.get_layer_scroller_owner(f.inner.occlusion.get().render_layer_id)
                == Some(f.scroller.id),
            owns_surface
        );
        assert_eq!(observes_fallback(), !owns_surface);
    }
}

#[test]
fn canvas_order_only_change_does_not_invalidate_local_paint() {
    let f = Fixture::new(3, true);
    f.cards[0].occlusion.update(|order| order.z_index += 10);
    f.engine.runtime_context.drain_node_effects();
    assert!(!f.any_dirty());
}

#[test]
fn first_canvas_query_on_a_component_observes_earlier_and_later_motion() {
    let mut f = Fixture::new(1, true);
    let root = f.engine.root_expanded_node.as_ref().unwrap().clone();
    let placement = root.canvas_transform_and_bounds.clone();
    let deps = [placement.untyped()];
    let sample = LocalProperty::computed(move || placement.get(), &deps);
    for offset in [17., 43., 6.] {
        root.get_common_properties()
            .borrow()
            .x
            .set(Some(Size::Pixels(offset.into())));
        f.settle();
        let transform = kurbo::Affine::from(sample.get().transform);
        assert_eq!(transform.as_coeffs()[4], offset);
    }
}

#[cfg(feature = "3d")]
#[test]
fn entering_and_leaving_spatial_placement_restores_conservative_drawing() {
    let mut f = Fixture::new(3, true);
    let fallback = f.inner.canvas_transform_and_bounds.untyped();
    let listener = f.inner.changed_listener.untyped();
    let observes_fallback =
        || pax_runtime::api::properties::property_has_direct_outbound(&fallback, &listener);
    assert!(!observes_fallback());
    // This observer keeps the original slot across both chart changes.
    let tab = f.cards[0].canvas_transform_and_bounds.clone();
    let deps = [tab.untyped()];
    let reads = Rc::new(std::cell::Cell::new(0));
    let observed = reads.clone();
    let sample = LocalProperty::computed(
        move || {
            observed.set(observed.get() + 1);
            tab.get()
        },
        &deps,
    );
    sample.get();
    f.outer
        .get_common_properties()
        .borrow()
        .rotate_x
        .set(Some(Rotation::Degrees(20.into())));
    f.settle();
    assert!(f.any_dirty());
    assert!(observes_fallback());
    assert!(!f.cards[0].has_surface_local_canvas_placement());
    let mut previous = sample.get();
    for x in [14, 26] {
        f.cards[0]
            .get_common_properties()
            .borrow()
            .x
            .set(Some(Size::Pixels(x.into())));
        f.settle();
        let current = sample.get();
        assert_ne!(current, previous);
        previous = current;
    }
    f.outer
        .get_common_properties()
        .borrow()
        .rotate_x
        .set(Some(Rotation::Degrees(0.into())));
    f.settle();
    sample.get();
    assert!(!observes_fallback());
    let before_host_motion = reads.get();
    f.clear();
    f.outer
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(33.into())));
    f.settle();
    assert!(f.cards[0].has_surface_local_canvas_placement());
    sample.get();
    assert_eq!(reads.get(), before_host_motion);
    assert!(!f.any_dirty());
}
