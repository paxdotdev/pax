use super::*;
use pax_runtime::api::pax_value::ToFromPaxAny;
use pax_runtime::api::{CommonProperties, Depth, Rotation};
use pax_runtime::{CommonPropertiesInit, ComponentInstance, PropertiesInit, PropertiesScopeInit};
use std::cell::RefCell;

fn args(common: CommonProperties) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(common.clone())))
        })),
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(LightSource::default().to_pax_any())))
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
        properties_scope: PropertiesScopeInit::None,
    }
}

#[cfg(feature = "3d")]
#[test]
fn light_uses_common_depth_once_and_inherits_reactive_ancestor_pose() {
    let context = Rc::new(crate::test_support::runtime_context());
    let parent = CommonProperties::default();
    parent.x.set(Some(Size::Pixels(10.into())));
    parent.z.set(Some(Depth::from(40.)));
    let child = CommonProperties::default();
    child.x.set(Some(Size::Pixels(20.into())));
    child.width.set(Some(Size::Pixels(4.into())));
    child.height.set(Some(Size::Pixels(6.into())));
    child.z.set(Some(Depth::from(50.)));
    let instance = LightSourceInstance::instantiate(args(child.clone()));
    let mut root_args = args(parent.clone());
    root_args.component_template = Some(RefCell::new(vec![instance.clone()]));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
    root.recurse_update(&context);
    context.drain_node_effects();
    let node = root.children.get()[0].clone();
    let resolve = || {
        instance
            .resolve_scene_light(&node, &context)
            .unwrap()
            .position
    };
    let point = resolve();
    assert_eq!((point.x, point.y, point.z), (32., 3., 90.));

    // Depth-only changes leave the XY footprint identical but must update lighting.
    child.z.set(Some(Depth::from(70.)));
    context.drain_node_effects();
    assert_eq!(resolve().z, 110.);

    parent.rotate_y.set(Some(Rotation::Degrees(90.into())));
    context.drain_node_effects();
    let point = resolve();
    assert!((point.x - 80.).abs() < 1e-9);
    assert!((point.z - 18.).abs() < 1e-9);

    parent.rotate_y.set(None);
    parent.z.set(None);
    child.z.set(None);
    context.drain_node_effects();
    assert_eq!(resolve().z, 0.);
}

#[test]
fn common_light_height_remains_available_in_2d_and_updates_reactively() {
    let context = Rc::new(crate::test_support::runtime_context());
    let child = CommonProperties::default();
    child.z.set(Some(Depth::from(50.)));
    let instance = LightSourceInstance::instantiate(args(child.clone()));
    let mut root_args = args(CommonProperties::default());
    root_args.component_template = Some(RefCell::new(vec![instance.clone()]));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
    root.recurse_update(&context);
    context.drain_node_effects();
    let node = root.children.get()[0].clone();
    assert_eq!(
        instance
            .resolve_scene_light(&node, &context)
            .unwrap()
            .position
            .z,
        50.
    );
    child.z.set(Some(Depth::from(75.)));
    context.drain_node_effects();
    assert_eq!(
        instance
            .resolve_scene_light(&node, &context)
            .unwrap()
            .position
            .z,
        75.
    );
    child.z.set(None);
    context.drain_node_effects();
    assert_eq!(
        instance
            .resolve_scene_light(&node, &context)
            .unwrap()
            .position
            .z,
        0.
    );
}
