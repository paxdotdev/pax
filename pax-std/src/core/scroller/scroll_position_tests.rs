use super::*;
use pax_runtime::api::math::Transform2;
use pax_runtime::api::pax_value::ToFromPaxAny;
use pax_runtime::api::{CommonProperties, Platform, TargetInfo, Variable, OS};
use pax_runtime::{
    CommonPropertiesInit, ComponentInstance, Globals, PropertiesInit, PropertiesScopeInit,
    RouteLocation, TransformAndBounds,
};

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
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
    }
}

fn mount() -> (Rc<RuntimeContext>, Rc<ExpandedNode>, ScrollerHost) {
    let context = Rc::new(RuntimeContext::new(Globals {
        elapsed_frames: Property::new(0),
        elapsed_millis: Property::new(0),
        viewport: Property::new(TransformAndBounds {
            transform: Transform2::identity(),
            bounds: (360.0, 120.0),
        }),
        gyro: Property::default(),
        accel: Property::default(),
        route_location: Property::new(RouteLocation::root()),
        browser_allows_scroller_vector_layers: Property::new(true),
        browser_allows_nested_scroller_vector_layers: Property::new(true),
        platform: Platform::Native,
        os: OS::IPhone,
        target: TargetInfo::new(Platform::Native, OS::IPhone),
        get_elapsed_millis: Rc::new(|| 0),
    }));
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
