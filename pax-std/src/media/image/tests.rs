use super::*;
use kurbo::Affine;
use kurbo::{BezPath, Rect};
use pax_runtime::api::pax_value::ToFromPaxAny;
use pax_runtime::api::{CommonProperties, Paint, ResolvedMaterial, ResolvedStroke, Size};
use pax_runtime::{CommonPropertiesInit, ComponentInstance, PropertiesInit, PropertiesScopeInit};

fn context() -> Rc<RuntimeContext> {
    Rc::new(crate::test_support::runtime_context())
}

fn args() -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(CommonProperties::default())))
        })),
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(Image::default().to_pax_any())))
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

#[derive(Default)]
struct RecordingRenderer {
    opacities: Vec<f64>,
    destinations: Vec<Rect>,
    loads: usize,
    subtree_opacity: bool,
    scopes: Vec<pax_runtime::api::OpacityScope>,
}

impl RenderContext for RecordingRenderer {
    fn supports_subtree_opacity(&self) -> bool {
        self.subtree_opacity
    }
    fn set_node_opacity_scopes(
        &mut self,
        _: usize,
        _: u32,
        scopes: &[pax_runtime::api::OpacityScope],
    ) {
        self.scopes = scopes.to_vec();
    }
    fn save(&mut self, _: usize) {}
    fn restore(&mut self, _: usize) {}
    fn clip(&mut self, _: usize, _: BezPath) {}
    fn layers(&self) -> usize {
        1
    }
    fn fill_with_opacity(&mut self, _: usize, _: BezPath, _: &Paint, _: f64) {
        unreachable!()
    }
    fn stroke_with_opacity(&mut self, _: usize, _: BezPath, _: &ResolvedStroke, _: f64) {
        unreachable!()
    }
    fn stroke_with_draw_range_and_material_and_opacity(
        &mut self,
        _: usize,
        _: BezPath,
        _: &ResolvedStroke,
        _: &ResolvedMaterial,
        _: f64,
        _: f64,
        _: f64,
    ) {
        unreachable!()
    }
    fn transform(&mut self, _: usize, _: Affine) {}
    fn load_image(&mut self, _: &str, _: &[u8], _: usize, _: usize) {
        self.loads += 1;
    }
    fn draw_image_with_opacity(&mut self, _: usize, _: &str, rect: Rect, opacity: f64) {
        self.destinations.push(rect);
        self.opacities.push(opacity);
    }
    fn get_image_size(&mut self, _: &str) -> Option<(usize, usize)> {
        unreachable!()
    }
    fn image_loaded(&self, _: &str) -> bool {
        unreachable!()
    }
    fn resize_layers_to(&mut self, _: usize, _: Rc<RefCell<Vec<bool>>>) {
        unreachable!()
    }
    fn clear(&mut self, _: usize) {
        unreachable!()
    }
    fn flush(&mut self, _: usize, _: Rc<RefCell<Vec<bool>>>) {
        unreachable!()
    }
    fn resize(&mut self, _: usize, _: usize) {
        unreachable!()
    }
    fn refresh_layers(&mut self, _: &[usize]) {
        unreachable!()
    }
}

fn mounted_image() -> (pax_runtime::PaxEngine, Rc<ImageInstance>, Rc<ExpandedNode>) {
    let mut engine = pax_runtime::PaxEngine {
        runtime_context: context(),
        root_expanded_node: None,
        scroller_tiling_policy: Default::default(),
    };
    engine.runtime_context.resize_canvas_layers_to(1);
    let image = ImageInstance::instantiate(args());
    let mut root_args = args();
    root_args.component_template = Some(RefCell::new(vec![image.clone()]));
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    let node = root.children.get()[0].clone();
    let properties = node.get_common_properties();
    properties.borrow().width.set(Some(Size::Pixels(80.into())));
    properties
        .borrow()
        .height
        .set(Some(Size::Pixels(80.into())));
    node.with_properties_unwrapped(|p: &mut Image| {
        p.source.set(ImageSource::Data(2, 1, vec![255; 8]))
    });
    engine.tick();
    (engine, image, node)
}

#[test]
fn fit_and_bounds_refit_the_loaded_image_and_a_new_source_replaces_it() {
    let (mut engine, image, node) = mounted_image();
    let mut renderer = RecordingRenderer::default();
    let ctx = engine.runtime_context.clone();
    for (fit, expected) in [
        (ImageFit::Fit, Rect::new(0., 20., 80., 60.)),
        (ImageFit::Paint, Rect::new(-40., 0., 120., 80.)),
        (ImageFit::Stretch, Rect::new(0., 0., 80., 80.)),
    ] {
        node.with_properties_unwrapped(|p: &mut Image| p.fit.set(fit));
        engine.tick();
        assert!(ctx.is_canvas_node_dirty(&node.id));
        image.render(&node, &ctx, &mut renderer);
        assert_eq!(renderer.destinations.last(), Some(&expected));
        assert_eq!(renderer.loads, 1);
        assert!(!ctx.is_canvas_node_dirty(&node.id));
    }
    node.get_common_properties()
        .borrow()
        .width
        .set(Some(Size::Pixels(60.into())));
    engine.tick();
    image.render(&node, &ctx, &mut renderer);
    assert_eq!(
        renderer.destinations.last(),
        Some(&Rect::new(0., 0., 60., 80.))
    );
    assert_eq!(renderer.loads, 1);

    node.with_properties_unwrapped(|p: &mut Image| p.fit.set(ImageFit::Fit));
    engine.tick();
    image.render(&node, &ctx, &mut renderer);
    assert_eq!(
        renderer.destinations.last(),
        Some(&Rect::new(0., 25., 60., 55.))
    );
    node.with_properties_unwrapped(|p: &mut Image| {
        p.source.set(ImageSource::Data(1, 2, vec![255; 8]))
    });
    engine.tick();
    image.render(&node, &ctx, &mut renderer);
    assert_eq!(
        renderer.destinations.last(),
        Some(&Rect::new(10., 0., 50., 80.))
    );
    assert_eq!(renderer.loads, 2);
}

#[cfg(feature = "3d")]
#[test]
fn parent_motion_replays_image_at_its_current_pose_with_the_same_local_paint() {
    use pax_runtime::api::Rotation;

    let (mut engine, image, node) = mounted_image();
    let ctx = engine.runtime_context.clone();
    let root = engine
        .root_expanded_node
        .as_ref()
        .unwrap()
        .get_common_properties();
    let mut renderer = RecordingRenderer::default();
    image.render(&node, &ctx, &mut renderer);
    let original = ctx.canvas_geometry_for_node(&node);
    for (x, tilt) in [(5., 0.), (150., 30.), (0., 0.)] {
        root.borrow().x.set(Some(Size::Pixels(x.into())));
        root.borrow()
            .rotate_y
            .set(Some(Rotation::Degrees(tilt.into())));
        let synchronous = node.canvas_transform_and_bounds.get();
        engine.tick();
        let current = ctx.canvas_geometry_for_node(&node);
        assert_eq!(
            current.surface_transform,
            Affine::from(synchronous.transform)
        );
        assert_eq!(current.spatial_geometry, node.presentation_geometry().get());
        assert_eq!(current.local_paint_revision, original.local_paint_revision);
        assert!(ctx.is_canvas_node_dirty(&node.id));
        image.render(&node, &ctx, &mut renderer);
        assert!(!ctx.is_canvas_node_dirty(&node.id));
    }
    assert_eq!(renderer.loads, 1);
    assert_eq!(renderer.destinations, vec![Rect::new(0., 20., 80., 60.); 4]);
}

#[test]
fn image_inherits_opacity_and_repaints_without_moving_or_reloading() {
    for subtree_opacity in [false, true] {
        let context = context();
        context.resize_canvas_layers_to(1);
        let parent_opacity = LocalProperty::new(Some(0.5.into()));
        let own_opacity = LocalProperty::new(Some(0.5.into()));
        let mut image_args = args();
        let own = own_opacity.clone();
        image_args.prototypical_common_properties =
            CommonPropertiesInit::Factory(Box::new(move |_, _| {
                Some(Rc::new(RefCell::new(CommonProperties {
                    opacity: own.clone(),
                    ..Default::default()
                })))
            }));
        image_args.prototypical_properties = PropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(
                Image {
                    source: Property::new(ImageSource::Data(1, 1, vec![255, 255, 255, 255])),
                    ..Default::default()
                }
                .to_pax_any(),
            )))
        }));
        let image = ImageInstance::instantiate(image_args);
        let mut root_args = args();
        root_args.component_template = Some(RefCell::new(vec![image.clone()]));
        let parent = parent_opacity.clone();
        root_args.prototypical_common_properties =
            CommonPropertiesInit::Factory(Box::new(move |_, _| {
                Some(Rc::new(RefCell::new(CommonProperties {
                    opacity: parent.clone(),
                    ..Default::default()
                })))
            }));
        let root =
            ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
        root.recurse_mount(&context);
        root.recurse_update(&context);
        let node = root.children.get()[0].clone();
        let initial_layout = node.transform_and_bounds.get();
        let mut renderer = RecordingRenderer {
            subtree_opacity,
            ..Default::default()
        };
        for (parent, own, expected) in [
            (0.5, 0.5, 0.25),
            (0.2, 0.5, 0.1),
            (0.2, 0.0, 0.0),
            (0.0, 0.5, 0.0),
            (0.0, 0.25, 0.0),
            (1.0, 1.0, 1.0),
        ] {
            parent_opacity.set(Some(parent.into()));
            own_opacity.set(Some(own.into()));
            root.recurse_update(&context);
            node.changed_listener.get();
            assert!(
                context.is_canvas_node_dirty(&node.id),
                "opacity must invalidate the retained image"
            );
            assert_eq!(node.transform_and_bounds.get(), initial_layout);
            image.render(&node, &context, &mut renderer);
            assert_eq!(
                renderer.opacities.last(),
                Some(&if subtree_opacity { 1.0 } else { expected })
            );
            if subtree_opacity {
                assert_eq!(
                    renderer.scopes,
                    vec![
                        pax_runtime::api::OpacityScope {
                            node_id: root.id.to_u32(),
                            opacity: parent as f32
                        },
                        pax_runtime::api::OpacityScope {
                            node_id: node.id.to_u32(),
                            opacity: own as f32
                        },
                    ]
                );
            }
            assert!(!context.is_canvas_node_dirty(&node.id));
        }
        assert_eq!(renderer.loads, 1, "opacity must not reload source pixels");
        assert_eq!(renderer.opacities.len(), 6);
    }
}
