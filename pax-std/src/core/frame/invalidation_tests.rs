use super::*;
use crate::core::scroller::{ScrollerHost, ScrollerHostInstance};
use crate::drawing::rectangle::{Rectangle, RectangleInstance};
use pax_runtime::api::pax_value::ToFromPaxAny;
use pax_runtime::api::{CommonProperties, Platform, Size, Variable, OS};
use pax_runtime::{CommonPropertiesInit, ComponentInstance, PropertiesInit, PropertiesScopeInit};

fn args<T: ToFromPaxAny + Clone + 'static>(props: T) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(CommonProperties::default())))
        })),
        prototypical_properties: PropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(props.clone().to_pax_any())))
        })),
        properties_scope: PropertiesScopeInit::Factory(Box::new(|_| Default::default())),
        handler_registry: None,
        children: None,
        component_template: None,
        component_settings: None,
        component_camera: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
    }
}

struct Fixture {
    context: Rc<RuntimeContext>,
    root: Rc<ExpandedNode>,
    frame: Rc<ExpandedNode>,
    nested: Rc<ExpandedNode>,
    props: Frame,
    leaves: Vec<Rc<ExpandedNode>>,
}

impl Fixture {
    fn mount() -> Self {
        let context = Rc::new(crate::test_support::runtime_context_with_target(
            (320.0, 240.0),
            Platform::Web,
            OS::Mac,
        ));
        let props = Frame::default();
        let scope = props.clone();
        let mut frame_args = args(props.clone());
        frame_args.properties_scope = PropertiesScopeInit::Factory(Box::new(move |_| {
            [
                (
                    "_clip_content".into(),
                    Variable::new_from_typed_property(scope._clip_content.clone()),
                ),
                (
                    "corner_radius".into(),
                    Variable::new_from_typed_property(scope.corner_radius.clone()),
                ),
            ]
            .into()
        }));
        let mut nested_args = args(Frame::default());
        nested_args.children = Some(RefCell::new(vec![RectangleInstance::instantiate(args(
            Rectangle::default(),
        ))]));
        frame_args.children = Some(RefCell::new(vec![
            RectangleInstance::instantiate(args(Rectangle::default())),
            FrameInstance::instantiate(nested_args),
        ]));
        let mut root_args = args(Frame::default());
        root_args.component_template =
            Some(RefCell::new(vec![FrameInstance::instantiate(frame_args)]));
        let root =
            ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
        root.recurse_update(&context);
        context.drain_node_effects();
        let frame = root.children.get()[0].clone();
        let nested = frame.children.get()[1].clone();
        let leaves = vec![
            frame.children.get()[0].clone(),
            nested.children.get()[0].clone(),
        ];
        // Give the nested subtree its own surface so the assertion catches
        // accidental invalidation across a retained surface boundary, too.
        leaves[1].occlusion.update(|o| o.render_layer_id = 1);
        context.dirty_canvases.borrow_mut().resize(2, true);
        context.drain_node_effects();
        let fixture = Self {
            context,
            root,
            frame,
            nested,
            props,
            leaves,
        };
        fixture.clear();
        fixture
    }

    fn clear(&self) {
        for id in self.context.dirty_canvas_node_ids() {
            self.context.clear_canvas_node_dirty(&id);
        }
        self.context.clear_all_dirty_canvases();
        self.context.take_native_messages();
    }

    fn patches(&self) -> Vec<FramePatch> {
        self.context.drain_node_effects();
        self.context
            .take_native_messages()
            .into_iter()
            .filter_map(|message| match message {
                pax_message::NativeMessage::FrameUpdate(patch)
                    if patch.id == self.frame.id.to_u32() =>
                {
                    Some(patch)
                }
                _ => None,
            })
            .collect()
    }

    fn assert_content_dirty(&self) {
        for leaf in &self.leaves {
            assert!(self.context.is_canvas_node_dirty(&leaf.id));
            assert!(self
                .context
                .is_canvas_dirty(&leaf.occlusion.get().render_layer_id));
        }
    }
}

#[test]
fn frame_order_only_patch_preserves_descendant_content() {
    let fixture = Fixture::mount();
    for z in [12, -4, 31] {
        fixture.frame.occlusion.update(|o| o.z_index = z);
        let patches = fixture.patches();
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].z_index, Some(z));
        assert!(patches[0].transform.is_none());
        assert!(patches[0].clip_path.is_none());
        assert!(!fixture.context.has_dirty_canvas_nodes());
        assert!(!fixture.context.has_dirty_canvas_layers());
    }
}

#[test]
fn nested_frame_order_only_patch_does_not_dirty_either_surface() {
    let fixture = Fixture::mount();
    fixture.nested.occlusion.update(|o| o.z_index = 15);
    fixture.context.drain_node_effects();
    assert!(fixture.context.take_native_messages().iter().any(|m| {
        matches!(m, pax_message::NativeMessage::FrameUpdate(p)
            if p.id == fixture.nested.id.to_u32() && p.z_index == Some(15))
    }));
    assert!(!fixture.context.has_dirty_canvas_nodes());
    assert!(!fixture.context.has_dirty_canvas_layers());
}

#[test]
fn frame_clip_changes_still_invalidate_descendant_content() {
    let fixture = Fixture::mount();
    fixture.props.corner_radius.set(12.0);
    assert!(fixture
        .patches()
        .iter()
        .any(|p| p.corner_radius == Some(12.0) && p.clip_path.is_some()));
    fixture.assert_content_dirty();
    fixture.clear();
    fixture.props._clip_content.set(false);
    assert!(fixture
        .patches()
        .iter()
        .any(|p| p.clip_content == Some(false)));
    fixture.assert_content_dirty();
}

#[test]
fn frame_geometry_changes_still_invalidate_descendant_content() {
    let fixture = Fixture::mount();
    let properties = fixture.frame.get_common_properties().borrow().clone();
    properties.x.set(Some(Size::Pixels(10.into())));
    properties.y.set(Some(Size::Pixels(20.into())));
    assert!(fixture.patches().iter().any(|p| p.transform.is_some()));
    fixture.assert_content_dirty();
    fixture.clear();
    properties.width.set(Some(Size::Pixels(180.into())));
    properties.height.set(Some(Size::Pixels(90.into())));
    assert!(fixture
        .patches()
        .iter()
        .any(|p| p.size_x == Some(180.0) && p.size_y == Some(90.0)));
    fixture.assert_content_dirty();
}

#[test]
fn frame_opacity_change_is_not_suppressed_by_a_simultaneous_order_change() {
    let fixture = Fixture::mount();
    fixture.frame.occlusion.update(|o| o.z_index = 17);
    fixture.frame.computed_opacity.set(0.5);
    assert!(fixture
        .patches()
        .iter()
        .any(|p| p.opacity == Some(0.5) && p.z_index == Some(17)));
    fixture.assert_content_dirty();
}

#[test]
fn frame_parent_change_is_observed_independently_of_order() {
    let fixture = Fixture::mount();
    fixture.frame.parent_frame.set(Some(fixture.root.id));
    assert!(fixture
        .patches()
        .iter()
        .any(|p| p.parent_frame == Some(Some(fixture.root.id.to_u32()))));
    fixture.assert_content_dirty();
}

#[test]
fn inserting_a_scroller_replays_the_shifted_surface_with_retained_paint() {
    let ctx = Rc::new(crate::test_support::runtime_context_with_target(
        (320.0, 240.0),
        Platform::Web,
        OS::Mac,
    ));
    let props = Frame::default();
    let mut frame = args(props.clone());
    frame.children = Some(RefCell::new(vec![RectangleInstance::instantiate(args(
        Rectangle::default(),
    ))]));
    let mut scroller = args(ScrollerHost::default());
    scroller.children = Some(RefCell::new(vec![FrameInstance::instantiate(frame)]));
    let mut app = args(Frame::default());
    app.component_template = Some(RefCell::new(vec![ScrollerHostInstance::instantiate(
        scroller,
    )]));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(app), &ctx);
    let settle = || {
        root.recurse_update(&ctx);
        for _ in 0..16 {
            ctx.drain_node_effects();
            if !ctx.take_occlusion_dirty() {
                ctx.prepare_scene_geometry();
                return;
            }
            pax_runtime::engine::occlusion::update_node_occlusion(&root, &ctx);
        }
        panic!("Frame placement did not settle");
    };
    settle();
    let host = root.children.get()[0].clone();
    let frame = host.children.get()[0].clone();
    let leaf = frame.children.get()[0].clone();
    let paint_revision = || ctx.canvas_geometry_for_node(&leaf).local_paint_revision;
    let mut previous = paint_revision();
    // An insertion ahead of this host shifts its slot. Repeat with a real
    // clip change on the same tick to exercise both invalidation paths.
    for clip_changed in [false, true] {
        let old = frame.occlusion.get().render_layer_id;
        assert_eq!(ctx.get_layer_scroller_owner(old), Some(host.id));
        ctx.take_canvas_node_removals();
        ctx.clear_canvas_node_dirty(&leaf.id);
        let mut children = root.children.get();
        children.extend(root.create_children_detached(
            [(
                ScrollerHostInstance::instantiate(args(ScrollerHost::default()))
                    as Rc<dyn InstanceNode>,
                root.stack.clone(),
            )],
            &ctx,
            &Rc::downgrade(&root),
        ));
        root.children
            .set(root.attach_children(children, &ctx, &root.parent_frame));
        if clip_changed {
            props.corner_radius.set(8.0);
        }
        settle();
        let new = frame.occlusion.get().render_layer_id;
        assert_eq!(new, old + 1);
        assert_eq!(ctx.get_layer_scroller_owner(new), Some(host.id));
        assert_eq!(leaf.occlusion.get().render_layer_id, new);
        assert!(ctx
            .take_canvas_node_removals()
            .contains(&(old, leaf.id.to_u32())));
        assert!(
            ctx.is_canvas_node_dirty(&leaf.id),
            "backend must replay the drawing"
        );
        let current = paint_revision();
        assert_eq!(current != previous, clip_changed);
        previous = current;
    }
}

#[test]
fn switching_clip_surfaces_dirties_paint_even_with_identical_geometry() {
    let f = Fixture::mount();
    let assign = |layer, owner| {
        f.context.clear_layer_scroller_owners();
        if let Some(id) = owner {
            f.context.register_layer_scroller_owner(layer, id);
        }
        for n in [&f.frame, &f.nested, &f.leaves[0], &f.leaves[1]] {
            n.occlusion.update(|o| {
                o.render_layer_id = layer;
                o.z_index += 1;
            });
        }
        f.context.drain_node_effects();
    };
    for (layer, owner) in [
        (1, Some(f.root.id)),
        (1, Some(f.nested.id)),
        (1, None),
        (0, None),
    ] {
        f.clear();
        assign(layer, owner);
        f.assert_content_dirty();
    }
}
