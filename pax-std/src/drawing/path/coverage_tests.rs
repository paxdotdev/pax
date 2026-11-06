use super::*;
use pax_runtime::api::{
    pax_value::ToFromPaxAny, Color, CommonProperties, Fill, Material, Paint, Size, StrokeCap,
    StrokeJoin, Variable,
};
use pax_runtime::{CommonPropertiesInit, ComponentInstance, PropertiesInit, PropertiesScopeInit};

fn args() -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(CommonProperties {
                width: LocalProperty::new(Some(Size::Pixels(100.into()))),
                height: LocalProperty::new(Some(Size::Pixels(100.into()))),
                ..Default::default()
            })))
        })),
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, _| {
            let path = Path::default();
            path.elements.set(vec![
                PathElement::Point(Size::Percent(0.into()), Size::Percent(0.into())),
                PathElement::Line,
                PathElement::Point(Size::Percent(100.into()), Size::Percent(100.into())),
                PathElement::Line,
                PathElement::Point(Size::Percent(100.into()), Size::Percent(0.into())),
                PathElement::Close,
            ]);
            path.fill.set(vec![Color::WHITE.into()]);
            path.stroke.set(vec![Color::BLACK.into()]);
            Some(Rc::new(RefCell::new(path.to_pax_any())))
        })),
        properties_scope: PropertiesScopeInit::Factory(Box::new(|props| {
            let props = props.borrow();
            let path = Path::ref_from_pax_any(&props).unwrap();
            [
                (
                    "elements".into(),
                    Variable::new_from_typed_property(path.elements.clone()),
                ),
                (
                    "fill".into(),
                    Variable::new_from_typed_property(path.fill.clone()),
                ),
                (
                    "stroke".into(),
                    Variable::new_from_typed_property(path.stroke.clone()),
                ),
                (
                    "smoothing".into(),
                    Variable::new_from_typed_property(path.smoothing.clone()),
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

#[test]
fn paint_changes_retain_path_coverage_until_its_shape_changes() {
    let context = Rc::new(crate::test_support::runtime_context());
    context.enable_work_profiling();
    let path = PathInstance::instantiate(args());
    let mut root_args = args();
    root_args.component_template = Some(RefCell::new(vec![path.clone(), path.clone()]));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
    let settle = || {
        root.recurse_update(&context);
        context.drain_node_effects();
        pax_runtime::engine::occlusion::update_node_occlusion(&root, &context);
    };
    settle();
    let nodes = root.children.get();
    let a = &nodes[0];
    let b = &nodes[1];
    let geometry_calls = |edit: &dyn Fn(), changed: bool| {
        context.begin_work_profile();
        edit();
        settle();
        let report = context.take_work_profile().unwrap();
        let calls = |id| {
            report
                .scopes
                .iter()
                .filter(|(node, name, _)| *node == id && *name == "local occlusion geometry")
                .map(|(_, _, work)| work.calls)
                .sum::<usize>()
        };
        assert_eq!(calls(a.id.to_u32()) > 0, changed);
        assert_eq!(
            calls(b.id.to_u32()),
            0,
            "repeated occurrences retain independent coverage"
        );
    };
    let before = context.occlusion_stats().records_updated;
    geometry_calls(
        &|| {
            a.with_properties_unwrapped(|p: &mut Path| {
                p.fill.set(vec![Color::RED.into()]);
                p.stroke.get()[0].paint.set(Paint::Solid(Color::BLUE));
            })
        },
        false,
    );
    assert!(context.occlusion_stats().records_updated > before);
    assert!(context.is_canvas_node_dirty(&a.id));
    geometry_calls(
        &|| {
            a.with_properties_unwrapped(|p: &mut Path| {
                p.fill.get()[0].opacity.set(0.4.into());
                p.fill.get()[0].material.set(Material::unlit());
            })
        },
        false,
    );
    assert!((path.resolve_coverage_opacity(a) - 0.4).abs() < 1e-6);

    geometry_calls(
        &|| {
            a.with_properties_unwrapped(|p: &mut Path| {
                p.stroke.get()[0].width.set(Size::Pixels(6.into()));
            })
        },
        true,
    );
    geometry_calls(
        &|| {
            a.with_properties_unwrapped(|p: &mut Path| {
                p.stroke.get()[0].cap.set(StrokeCap::Round);
                p.stroke.get()[0].join.set(StrokeJoin::Round);
            })
        },
        true,
    );
    geometry_calls(
        &|| {
            a.with_properties_unwrapped(|p: &mut Path| {
                p.fill.get()[0].paint.set(Paint::Solid(Color::TRANSPARENT));
            })
        },
        true,
    );
    geometry_calls(
        &|| a.with_properties_unwrapped(|p: &mut Path| p.stroke.set(vec![])),
        true,
    );
    assert!(path.resolve_local_occlusion_path(a).is_none());
    geometry_calls(
        &|| {
            a.with_properties_unwrapped(|p: &mut Path| {
                p.fill.set(vec![Fill::from(Color::WHITE)]);
            })
        },
        true,
    );
    geometry_calls(
        &|| {
            a.get_common_properties()
                .borrow()
                .width
                .set(Some(Size::Pixels(60.into())))
        },
        true,
    );
    geometry_calls(
        &|| {
            a.with_properties_unwrapped(|p: &mut Path| {
                p.elements.set(Path::start(
                    Size::Pixels(20.into()),
                    Size::Pixels(20.into()),
                ));
            })
        },
        true,
    );
    geometry_calls(&|| context.mark_node_occlusion_dirty(a.id.to_u32()), true);
    root.recurse_unmount(&context);
}
