use super::*;
use crate::drawing::rectangle::{Rectangle, RectangleInstance};
use pax_runtime::api::pax_value::ToFromPaxAny;
use pax_runtime::api::{CommonProperties, Platform, Variable, OS};
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
    fixture.frame.transform_and_bounds.update(|tab| {
        tab.transform =
            pax_runtime::api::math::Transform2::new(Affine::translate((10.0, 20.0)).as_coeffs());
    });
    assert!(fixture.patches().iter().any(|p| p.transform.is_some()));
    fixture.assert_content_dirty();
    fixture.clear();
    fixture.frame.transform_and_bounds.update(|tab| {
        tab.bounds = (180.0, 90.0);
    });
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
