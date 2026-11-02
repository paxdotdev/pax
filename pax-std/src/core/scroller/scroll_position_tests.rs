use super::*;
use pax_runtime::api::pax_value::ToFromPaxAny;
use pax_runtime::api::{CommonProperties, Platform, Variable, OS};
use pax_runtime::{CommonPropertiesInit, ComponentInstance, PropertiesInit, PropertiesScopeInit};

fn args(props: ScrollerHost) -> InstantiationArgs {
    let scope = props.clone();
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(CommonProperties::default())))
        })),
        prototypical_properties: PropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(props.clone().to_pax_any())))
        })),
        properties_scope: PropertiesScopeInit::Factory(Box::new(move |_| {
            [
                (
                    "scroll_pos_x".into(),
                    Variable::new_from_typed_property(scope.scroll_pos_x.clone()),
                ),
                (
                    "scroll_pos_y".into(),
                    Variable::new_from_typed_property(scope.scroll_pos_y.clone()),
                ),
                (
                    "scroll_width".into(),
                    Variable::new_from_typed_property(scope.scroll_width.clone()),
                ),
                (
                    "scroll_height".into(),
                    Variable::new_from_typed_property(scope.scroll_height.clone()),
                ),
                (
                    "_presentation_scroll_x".into(),
                    Variable::new_from_typed_property(scope._presentation_scroll_x.clone()),
                ),
                (
                    "_presentation_scroll_y".into(),
                    Variable::new_from_typed_property(scope._presentation_scroll_y.clone()),
                ),
            ]
            .into()
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
    }
}

fn mount() -> (Rc<RuntimeContext>, Rc<ExpandedNode>, ScrollerHost) {
    let context = Rc::new(crate::test_support::runtime_context_with_target(
        (360.0, 120.0),
        Platform::Native,
        OS::IPhone,
    ));
    let props = ScrollerHost {
        scroll_pos_x: Property::new(3916.0),
        scroll_pos_y: Property::new(4036.0),
        scroll_width: Property::new(Size::Pixels(8192.into())),
        scroll_height: Property::new(Size::Pixels(8192.into())),
        ..Default::default()
    };
    let host = ScrollerHostInstance::instantiate(args(props.clone()));
    let mut root_args = args(ScrollerHost::default());
    root_args.component_template = Some(RefCell::new(vec![host]));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
    root.recurse_update(&context);
    context.drain_node_effects();
    (context, root, props)
}

fn assert_position(context: &RuntimeContext, root: &ExpandedNode, position: (f64, f64)) {
    let node = root.children.get()[0].clone();
    let state = context
        .get_scroller_surface_state(node.id.to_u32())
        .unwrap();
    assert_eq!(
        (state.presentation_scroll_x, state.presentation_scroll_y),
        position
    );
    assert_eq!(effective_presentation_scroll(&node, context), position);
    assert_eq!(
        node.instance_node.borrow().resolve_scroll_offset(&node),
        Some(position)
    );
    let patches: Vec<_> = context
        .take_native_messages()
        .into_iter()
        .filter_map(|message| {
            if let pax_message::NativeMessage::ScrollerUpdate(patch) = message {
                Some(patch)
            } else {
                None
            }
        })
        .collect();
    assert!(patches
        .iter()
        .any(|patch| patch.presentation_scroll_x == Some(position.0)));
    assert!(patches
        .iter()
        .any(|patch| patch.presentation_scroll_y == Some(position.1)));
}

#[test]
fn initial_center_reaches_host_and_renderer_before_native_scroll() {
    let (context, root, _) = mount();
    assert_position(&context, &root, (3916.0, 4036.0));
}

#[test]
fn programmatic_center_and_zoom_replace_last_native_offset() {
    let (context, root, props) = mount();
    context.take_native_messages();
    let id = root.children.get()[0].id.to_u32();
    // A native pan updates the cache before its two-way bound properties.
    context.update_scroller_surface_scroll(id, 32.0, 96.0, 32.0, 96.0);
    props.scroll_pos_x.set(32.0);
    props.scroll_pos_y.set(96.0);
    context.drain_node_effects();
    assert_position(&context, &root, (32.0, 96.0));
    props.scroll_pos_x.set(3916.0);
    props.scroll_pos_y.set(4036.0);
    context.drain_node_effects();
    assert_position(&context, &root, (3916.0, 4036.0));
    // Zoom changes content extent and position in the same frame.
    props.scroll_width.set(Size::Pixels(16384.into()));
    props.scroll_height.set(Size::Pixels(16384.into()));
    props.scroll_pos_x.set(8012.0);
    props.scroll_pos_y.set(8132.0);
    context.drain_node_effects();
    assert_position(&context, &root, (8012.0, 8132.0));
}

#[test]
fn programmatic_scroll_preserves_host_presentation_delta() {
    let (context, root, props) = mount();
    context.take_native_messages();
    let id = root.children.get()[0].id.to_u32();
    context.update_scroller_surface_scroll(id, 3916.0, 4036.0, 3924.0, 4048.0);
    props.scroll_pos_x.set(4016.0);
    props.scroll_pos_y.set(4236.0);
    context.drain_node_effects();
    assert_position(&context, &root, (4024.0, 4248.0));
}

fn observe(context: &RuntimeContext, root: &ExpandedNode, x: f64, y: f64) {
    let node = root.children.get()[0].clone();
    context.update_scroller_surface_scroll(node.id.to_u32(), x, y, x, y);
    node.instance_node.borrow().handle_native_interrupt(
        &node,
        &NativeInterrupt::ScrollerPosition(pax_message::ScrollerPositionInterruptArgs {
            id: node.id.to_u32(),
            scroll_x: x,
            scroll_y: y,
            presentation_scroll_x: Some(x),
            presentation_scroll_y: Some(y),
        }),
    );
}

fn updates(context: &RuntimeContext) -> Vec<ScrollerPatch> {
    context
        .take_native_messages()
        .into_iter()
        .filter_map(|message| match message {
            pax_message::NativeMessage::ScrollerUpdate(patch) => Some(patch),
            _ => None,
        })
        .collect()
}

#[test]
fn native_observations_update_presentation_without_emitting_scroll_commands() {
    let (context, root, props) = mount();
    context.take_native_messages();
    for (x, y) in [(30.0, 100.0), (45.0, 140.0), (45.0, 110.0)] {
        observe(&context, &root, x, y);
        context.drain_node_effects();
        assert_eq!((props.scroll_pos_x.get(), props.scroll_pos_y.get()), (x, y));
        let patches = updates(&context);
        assert!(!patches.is_empty());
        assert!(patches
            .iter()
            .all(|p| p.scroll_x.is_none() && p.scroll_y.is_none()));
        assert!(patches.iter().any(|p| p.presentation_scroll_y == Some(y)));
    }
}

#[test]
fn authored_write_after_observation_is_a_command_only_on_its_axis() {
    let (context, root, props) = mount();
    context.take_native_messages();
    observe(&context, &root, 30.0, 100.0);
    props.scroll_pos_y.set(600.0);
    context.drain_node_effects();
    let patches = updates(&context);
    assert!(patches.iter().all(|p| p.scroll_x.is_none()));
    assert!(patches.iter().any(|p| p.scroll_y == Some(600.0)));
    // The command's native acknowledgement must not become another command.
    observe(&context, &root, 30.0, 600.0);
    context.drain_node_effects();
    assert!(updates(&context)
        .iter()
        .all(|p| p.scroll_x.is_none() && p.scroll_y.is_none()));
    props.scroll_pos_y.set(800.0);
    context.drain_node_effects();
    assert!(updates(&context).iter().any(|p| p.scroll_y == Some(800.0)));
}

mod hit_testing {
    use super::*;
    use pax_runtime::api::math::{Point2, Transform2};
    use pax_runtime::{InstanceNodePtr, TransformAndBounds};

    fn geometry(node: &ExpandedNode, bounds: (f64, f64), transform: Affine) {
        node.transform_and_bounds
            .replace_with(LocalProperty::new(TransformAndBounds {
                transform: Transform2::new(transform.as_coeffs()),
                bounds,
            }));
    }

    fn hit(
        context: &RuntimeContext,
        node: &Rc<ExpandedNode>,
        ray: Point2<pax_runtime::api::Window>,
    ) -> bool {
        context
            .get_elements_beneath_ray(Some(node.clone()), ray, true, vec![], true)
            .iter()
            .any(|hit| hit.id == node.id)
    }

    fn rounded_host() -> (Rc<RuntimeContext>, Rc<ExpandedNode>, ScrollerHost) {
        let (context, root, props) = mount();
        let node = root.children.get()[0].clone();
        geometry(&node, (100.0, 100.0), Affine::IDENTITY);
        node.with_properties_unwrapped(|props: &mut ScrollerHost| props.corner_radius.set(32.0));
        context.drain_node_effects();
        (context, node, props)
    }

    #[test]
    fn rounded_hit_region_matches_rendered_clip() {
        let (context, node, _) = rounded_host();
        for radius in [32.0, 1000.0] {
            node.with_properties_unwrapped(|props: &mut ScrollerHost| {
                props.corner_radius.set(radius)
            });
            context.drain_node_effects();
            let clip = node
                .instance_node
                .borrow()
                .resolve_effect_clip_path(&node)
                .unwrap();
            for (x, y, expected) in [(4.0, 4.0, false), (50.0, 50.0, true), (96.0, 4.0, false)] {
                assert_eq!(clip.contains(kurbo::Point::new(x, y)), expected);
                assert_eq!(
                    hit(&context, &node, Point2::new(x, y)),
                    expected,
                    "point ({x}, {y}) at radius {radius}"
                );
            }
        }
        // Preserve the existing open rectangular boundary for input.
        for (x, y) in [(0.0, 50.0), (100.0, 50.0), (50.0, 0.0), (50.0, 100.0)] {
            assert!(!hit(&context, &node, Point2::new(x, y)));
        }
    }

    #[test]
    fn zero_radius_and_disabled_clipping_keep_rectangular_hits() {
        let (context, node, _) = rounded_host();
        for radius in [0.0, -10.0] {
            node.with_properties_unwrapped(|props: &mut ScrollerHost| {
                props.corner_radius.set(radius)
            });
            context.drain_node_effects();
            assert!(hit(&context, &node, Point2::new(4.0, 4.0)));
        }
        node.with_properties_unwrapped(|props: &mut ScrollerHost| {
            props.corner_radius.set(32.0);
            props._clip_content.set(false);
        });
        context.drain_node_effects();
        assert!(node
            .instance_node
            .borrow()
            .resolve_effect_clip_path(&node)
            .is_none());
        assert!(hit(&context, &node, Point2::new(4.0, 4.0)));
        assert!(!hit(&context, &node, Point2::new(-1.0, 4.0)));
    }

    #[test]
    fn rounded_hits_follow_rotated_scaled_host_coordinates() {
        let (context, node, _) = rounded_host();
        let transform =
            Affine::translate((250.0, 90.0)) * Affine::rotate(0.35) * Affine::scale(1.4);
        geometry(&node, (100.0, 100.0), transform);
        for (x, y, expected) in [(4.0, 4.0, false), (25.0, 25.0, true), (50.0, 50.0, true)] {
            let ray = transform * Point2::new(x, y);
            assert_eq!(hit(&context, &node, ray), expected);
        }
    }

    struct HitLeaf {
        base: BaseInstance,
    }
    impl InstanceNode for HitLeaf {
        fn instantiate(args: InstantiationArgs) -> Rc<Self> {
            Rc::new(Self {
                base: BaseInstance::new(
                    args,
                    InstanceFlags {
                        invisible_to_slot: false,
                        invisible_to_raycasting: false,
                        layer: Layer::Canvas,
                        is_component: false,
                        is_slot: false,
                    },
                ),
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
            f.debug_struct("HitLeaf").finish()
        }
    }

    #[test]
    fn nested_scrolled_hit_testing_respects_rounded_clip_and_unclippable_escape() {
        let context = Rc::new(crate::test_support::runtime_context_with_target(
            (800.0, 800.0),
            Platform::Web,
            OS::Unknown,
        ));
        let props = ScrollerHost {
            corner_radius: Property::new(32.0),
            scroll_pos_y: Property::new(20.0),
            scroll_width: Property::new(Size::Pixels(1000.into())),
            scroll_height: Property::new(Size::Pixels(1000.into())),
            ..Default::default()
        };
        let leaf: InstanceNodePtr = HitLeaf::instantiate(args(ScrollerHost::default()));
        let mut inner_args = args(props.clone());
        inner_args.children = Some(RefCell::new(vec![leaf]));
        let inner: InstanceNodePtr = ScrollerHostInstance::instantiate(inner_args);
        let mut outer_args = args(ScrollerHost {
            scroll_pos_y: Property::new(40.0),
            scroll_width: Property::new(Size::Pixels(1000.into())),
            scroll_height: Property::new(Size::Pixels(1000.into())),
            ..Default::default()
        });
        outer_args.children = Some(RefCell::new(vec![inner]));
        let outer: InstanceNodePtr = ScrollerHostInstance::instantiate(outer_args);
        let mut root_args = args(ScrollerHost::default());
        root_args.component_template = Some(RefCell::new(vec![outer]));
        let root =
            ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
        root.recurse_update(&context);
        context.drain_node_effects();
        let outer = root.children.get()[0].clone();
        let inner = outer.children.get()[0].clone();
        let leaf = inner.children.get()[0].clone();
        // Layout remains in content coordinates; native presentation moves descendants.
        let outer_transform =
            Affine::translate((100.0, 80.0)) * Affine::rotate(0.2) * Affine::scale(1.2);
        let inner_transform = outer_transform * Affine::translate((40.0, 100.0));
        geometry(&outer, (300.0, 300.0), outer_transform);
        geometry(&inner, (100.0, 100.0), inner_transform);
        geometry(&leaf, (300.0, 300.0), inner_transform);
        context.update_scroller_surface_scroll(outer.id.to_u32(), 0.0, 40.0, 0.0, 40.0);
        context.update_scroller_surface_scroll(inner.id.to_u32(), 0.0, 20.0, 0.0, 20.0);
        let presented_inner = outer_transform * Affine::translate((40.0, 60.0));
        let outside = presented_inner * Point2::new(4.0, 4.0);
        let inside = presented_inner * Point2::new(25.0, 25.0);
        let hit = |ray| {
            context
                .get_elements_beneath_ray(Some(root.clone()), ray, true, vec![], false)
                .into_iter()
                .next()
                .map(|node| node.id)
        };
        assert_eq!(hit(inside), Some(leaf.id));
        assert_eq!(hit(outside), None);
        leaf.get_common_properties()
            .borrow()
            .unclippable
            .set(Some(true));
        assert_eq!(hit(outside), Some(leaf.id));
        leaf.get_common_properties()
            .borrow()
            .unclippable
            .set(Some(false));
        inner.with_properties_unwrapped(|props: &mut ScrollerHost| props._clip_content.set(false));
        context.drain_node_effects();
        assert_eq!(hit(outside), Some(leaf.id));
    }
}
