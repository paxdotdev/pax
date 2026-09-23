use super::*;
use pax_runtime::api::math::Transform2;
use pax_runtime::api::pax_value::ToFromPaxAny;
use pax_runtime::api::{CommonProperties, TargetInfo, OS};
use pax_runtime::{
    CommonPropertiesInit, ComponentInstance, Globals, PropertiesInit, PropertiesScopeInit,
    RouteLocation, TransformAndBounds,
};
use std::cell::RefCell;

fn args(spacer: DynamicIslandSpacer) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(CommonProperties::default())))
        })),
        prototypical_properties: PropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(spacer.clone().to_pax_any())))
        })),
        handler_registry: None,
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

fn mount(
    platform: Platform,
    os: OS,
) -> (Rc<RuntimeContext>, Rc<ExpandedNode>, DynamicIslandSpacer) {
    mount_in_container(platform, os, false)
}

fn mount_in_container(
    platform: Platform,
    os: OS,
    stacked: bool,
) -> (Rc<RuntimeContext>, Rc<ExpandedNode>, DynamicIslandSpacer) {
    let context = Rc::new(RuntimeContext::new(Globals {
        elapsed_frames: Property::new(0),
        elapsed_millis: Property::new(0),
        viewport: Property::new(TransformAndBounds {
            transform: Transform2::identity(),
            bounds: (440.0, 956.0),
        }),
        gyro: Property::default(),
        accel: Property::default(),
        route_location: Property::new(RouteLocation::root()),
        browser_allows_scroller_vector_layers: Property::new(true),
        browser_allows_nested_scroller_vector_layers: Property::new(true),
        platform,
        os,
        target: TargetInfo::new(platform, os),
        get_elapsed_millis: Rc::new(|| 0),
    }));
    let spacer = DynamicIslandSpacer::default();
    let mut instance: Rc<dyn InstanceNode> =
        DynamicIslandSpacerInstance::instantiate(args(spacer.clone()));
    if stacked {
        use crate::layout::stacker::{Stacker, StackerInstance};
        use pax_runtime::api::Size;
        let mut stack_args = args(DynamicIslandSpacer::default());
        stack_args.children = Some(RefCell::new(vec![instance]));
        stack_args.prototypical_properties = PropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(
                Stacker {
                    autosize: Property::new(true),
                    ..Default::default()
                }
                .to_pax_any(),
            )))
        }));
        stack_args.prototypical_common_properties =
            CommonPropertiesInit::Factory(Box::new(|_, _| {
                Some(Rc::new(RefCell::new(CommonProperties {
                    width: Property::new(Some(Size::Percent(100.into()))),
                    ..Default::default()
                })))
            }));
        instance = StackerInstance::instantiate(stack_args);
    }
    let mut root_args = args(DynamicIslandSpacer::default());
    root_args.component_template = Some(RefCell::new(vec![instance]));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
    root.recurse_update(&context);
    context.drain_node_effects();
    (context, root, spacer)
}

#[test]
fn dynamic_island_spacer_reserves_space_in_autosized_stacker() {
    let (context, root, _) = mount_in_container(Platform::Native, OS::IPhone, true);
    let stack = root.children.get()[0].clone();
    context.set_safe_area_insets(SafeAreaInsets {
        top: 62.0,
        ..Default::default()
    });
    context.drain_node_effects();
    assert_eq!(stack.transform_and_bounds.get().bounds, (440.0, 62.0));
    context.set_safe_area_insets(SafeAreaInsets::default());
    context.drain_node_effects();
    assert_eq!(stack.transform_and_bounds.get().bounds, (440.0, 0.0));
}

#[test]
fn dynamic_island_spacer_tracks_rotation_edges_and_window_resize() {
    for os in [OS::IPhone, OS::IPad] {
        let (context, root, spacer) = mount(Platform::Native, os);
        let node = root.children.get()[0].clone();
        assert_eq!(node.transform_and_bounds.get().bounds, (440.0, 0.0));
        context.set_safe_area_insets(SafeAreaInsets {
            top: 62.0,
            bottom: 34.0,
            ..Default::default()
        });
        context.drain_node_effects();
        assert_eq!(spacer.inset.get(), 62.0);
        assert_eq!(node.transform_and_bounds.get().bounds, (440.0, 62.0));
        assert_eq!(root.transform_and_bounds.get().bounds, (440.0, 956.0));

        context.edit_globals(|globals| {
            globals
                .viewport
                .update(|viewport| viewport.bounds = (956.0, 440.0));
        });
        context.set_safe_area_insets(SafeAreaInsets {
            top: 0.0,
            right: 62.0,
            bottom: 21.0,
            left: 62.0,
        });
        context.drain_node_effects();
        assert_eq!(node.transform_and_bounds.get().bounds, (956.0, 0.0));
        assert_eq!(spacer.inset.get(), 0.0);
        for (edge, size, inset) in [
            (SafeAreaEdge::Left, (62.0, 440.0), 62.0),
            (SafeAreaEdge::Right, (62.0, 440.0), 62.0),
            (SafeAreaEdge::Bottom, (956.0, 21.0), 21.0),
        ] {
            spacer.edge.set(edge);
            context.drain_node_effects();
            assert_eq!(node.transform_and_bounds.get().bounds, size);
            assert_eq!(spacer.inset.get(), inset);
        }
    }
}

#[test]
fn dynamic_island_spacer_is_zero_on_unsupported_chassis() {
    for (platform, os) in [
        (Platform::Web, OS::IPhone),
        (Platform::Native, OS::Mac),
        (Platform::Native, OS::Android),
    ] {
        let (context, root, spacer) = mount(platform, os);
        context.set_safe_area_insets(SafeAreaInsets {
            top: 62.0,
            left: 62.0,
            ..Default::default()
        });
        context.drain_node_effects();
        let node = root.children.get()[0].clone();
        assert_eq!(node.transform_and_bounds.get().bounds, (440.0, 0.0));
        assert_eq!(spacer.inset.get(), 0.0);
        spacer.edge.set(SafeAreaEdge::Left);
        context.drain_node_effects();
        assert_eq!(node.transform_and_bounds.get().bounds, (0.0, 956.0));
    }
}

#[test]
fn dynamic_island_spacer_rejects_invalid_native_geometry() {
    let (context, root, _) = mount(Platform::Native, OS::IPhone);
    context.set_safe_area_insets(SafeAreaInsets {
        top: f64::NAN,
        right: f64::INFINITY,
        bottom: -34.0,
        left: 12.0,
    });
    context.drain_node_effects();
    assert_eq!(
        context.safe_area_insets().get(),
        SafeAreaInsets {
            left: 12.0,
            ..Default::default()
        }
    );
    assert_eq!(
        root.children.get()[0].transform_and_bounds.get().bounds,
        (440.0, 0.0)
    );
}
