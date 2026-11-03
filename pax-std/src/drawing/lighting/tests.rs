use super::*;
use pax_runtime::api::pax_value::ToFromPaxAny;
use pax_runtime::api::{CommonProperties, Depth, Rotation};
use pax_runtime::{CommonPropertiesInit, ComponentInstance, PropertiesInit, PropertiesScopeInit};
use std::cell::RefCell;

fn args(common: CommonProperties) -> InstantiationArgs {
    args_with_properties(common, LightSource::default())
}

fn args_with_properties<P: ToFromPaxAny + Clone + 'static>(
    common: CommonProperties,
    properties: P,
) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(common.clone())))
        })),
        prototypical_properties: PropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(properties.clone().to_pax_any())))
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

#[test]
fn strip_is_one_light_primitive_with_reactive_height_and_no_wrapper() {
    let context = Rc::new(crate::test_support::runtime_context());
    let common = CommonProperties::default();
    common.x.set(Some(Size::Pixels(10.into())));
    common.y.set(Some(Size::Pixels(20.into())));
    common.width.set(Some(Size::Pixels(80.into())));
    common.height.set(Some(Size::Pixels(2.into())));
    common.z.set(Some(Depth::from(58.)));
    let properties = StripLight::default();
    let instance =
        StripLightInstance::instantiate(args_with_properties(common.clone(), properties.clone()));
    assert!(instance.has_scene_lighting());
    let mut root_args = args(CommonProperties::default());
    root_args.component_template = Some(RefCell::new(vec![instance.clone()]));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
    root.recurse_update(&context);
    context.drain_node_effects();
    let node = root.children.get()[0].clone();
    assert!(node.children.get().is_empty());
    let resolve = || instance.resolve_scene_light(&node, &context).unwrap();
    let light = resolve();
    assert_eq!(light.shape, LightShape::Strip);
    assert_eq!(light.position, Vector3::new(50., 21., 58.));
    assert_eq!(light.half_extent, Vector3::new(40., 0., 0.));
    assert_eq!(context.collect_scene_lighting_for_layer(0).lights.len(), 1);

    common.z.set(Some(Depth::from(75.)));
    properties.intensity.set(2.);
    context.drain_node_effects();
    assert_eq!(resolve().position.z, 75.);
    assert_eq!(resolve().intensity, 2.);
    let collected = context.collect_scene_lighting_for_layer(0);
    assert_eq!(collected.lights.len(), 1);
    assert_eq!(collected.lights[0].position.z, 75.);
    assert_eq!(collected.lights[0].intensity, 2.);

    properties.enabled.set(false);
    context.drain_node_effects();
    assert!(instance.resolve_scene_light(&node, &context).is_none());
    assert!(context
        .collect_scene_lighting_for_layer(0)
        .lights
        .is_empty());
    properties.enabled.set(true);
    common.z.set(None);
    common.width.set(Some(Size::Pixels(0.into())));
    context.drain_node_effects();
    assert_eq!(resolve().position.z, 0.);
    assert_eq!(resolve().half_extent, Vector3::default());
    assert_eq!(context.collect_scene_lighting_for_layer(0).lights.len(), 1);
}

#[test]
fn configurable_light_keeps_reactive_shape_and_direction() {
    let context = Rc::new(crate::test_support::runtime_context());
    let common = CommonProperties::default();
    common.width.set(Some(Size::Pixels(80.into())));
    common.height.set(Some(Size::Pixels(2.into())));
    common.z.set(Some(Depth::from(58.)));
    let properties = LightSource::default();
    let instance =
        LightSourceInstance::instantiate(args_with_properties(common, properties.clone()));
    let mut root_args = args(CommonProperties::default());
    root_args.component_template = Some(RefCell::new(vec![instance.clone()]));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
    root.recurse_update(&context);
    context.drain_node_effects();
    let node = root.children.get()[0].clone();
    let resolve = || instance.resolve_scene_light(&node, &context).unwrap();
    assert_eq!(resolve().shape, LightShape::Point);
    assert_eq!(resolve().half_extent, Vector3::default());
    properties.shape.set(LightShape::Strip);
    context.drain_node_effects();
    assert_eq!(resolve().shape, LightShape::Strip);
    assert_eq!(resolve().half_extent, Vector3::new(40., 0., 0.));
    properties.shape.set(LightShape::Directional);
    properties.direction.set(Vector3::new(1., 2., -3.));
    context.drain_node_effects();
    let light = resolve();
    assert_eq!(light.shape, LightShape::Directional);
    assert_eq!(light.half_extent, Vector3::default());
    assert_eq!(light.direction, Vector3::new(1., 2., -3.));
}

#[cfg(feature = "3d")]
#[test]
fn strip_inherits_spatial_depth_once_and_rotates_its_endpoints() {
    let context = Rc::new(crate::test_support::runtime_context());
    let parent = CommonProperties::default();
    parent.z.set(Some(Depth::from(40.)));
    let child = CommonProperties::default();
    child.width.set(Some(Size::Pixels(100.into())));
    child.height.set(Some(Size::Pixels(2.into())));
    child.z.set(Some(Depth::from(58.)));
    let instance =
        StripLightInstance::instantiate(args_with_properties(child.clone(), StripLight::default()));
    let mut root_args = args(parent.clone());
    root_args.component_template = Some(RefCell::new(vec![instance.clone()]));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
    root.recurse_update(&context);
    context.drain_node_effects();
    let node = root.children.get()[0].clone();
    let resolve = || instance.resolve_scene_light(&node, &context).unwrap();
    assert_eq!(resolve().position, Vector3::new(50., 1., 98.));

    parent.rotate_y.set(Some(Rotation::Degrees(90.into())));
    context.drain_node_effects();
    let light = resolve();
    assert!((light.position.x - 58.).abs() < 1e-9);
    assert!((light.position.z + 10.).abs() < 1e-9);
    assert!(light.half_extent.x.abs() < 1e-9);
    assert!((light.half_extent.z + 50.).abs() < 1e-9);

    child.z.set(Some(Depth::from(75.)));
    context.drain_node_effects();
    assert!((resolve().position.x - 75.).abs() < 1e-9);
    assert!((resolve().position.z + 10.).abs() < 1e-9);
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
