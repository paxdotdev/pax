use super::*;
use kurbo::{BezPath, Rect};
use pax_runtime::api::math::Transform2;
use pax_runtime::api::pax_value::ToFromPaxAny;
use pax_runtime::api::{CommonProperties, Fill, Material, Platform, Stroke, TargetInfo, OS};
use pax_runtime::{
    CommonPropertiesInit, ComponentInstance, Globals, PropertiesInit, PropertiesScopeInit,
    RouteLocation, TransformAndBounds,
};

fn context() -> Rc<RuntimeContext> {
    Rc::new(RuntimeContext::new(Globals {
        elapsed_frames: Property::new(0),
        elapsed_millis: Property::new(0),
        viewport: Property::new(TransformAndBounds {
            transform: Transform2::identity(),
            bounds: (100.0, 100.0),
        }),
        gyro: Property::default(),
        accel: Property::default(),
        route_location: Property::new(RouteLocation::root()),
        browser_allows_scroller_vector_layers: Property::new(true),
        browser_allows_nested_scroller_vector_layers: Property::new(true),
        platform: Platform::Unknown,
        os: OS::Unknown,
        target: TargetInfo::new(Platform::Unknown, OS::Unknown),
        get_elapsed_millis: Rc::new(|| 0),
    }))
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
    fn fill_with_opacity(&mut self, _: usize, _: BezPath, _: &Fill, _: f64) {
        unreachable!()
    }
    fn stroke_with_opacity(&mut self, _: usize, _: BezPath, _: &Stroke, _: f64) {
        unreachable!()
    }
    fn stroke_with_draw_range_and_material_and_opacity(
        &mut self,
        _: usize,
        _: BezPath,
        _: &Stroke,
        _: &Material,
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
    fn draw_image_with_opacity(&mut self, _: usize, _: &str, _: Rect, opacity: f64) {
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

#[test]
fn image_inherits_opacity_and_repaints_without_moving_or_reloading() {
    for subtree_opacity in [false, true] {
        let context = context();
        context.resize_canvas_layers_to(1);
        let parent_opacity = Property::new(Some(0.5.into()));
        let own_opacity = Property::new(Some(0.5.into()));
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
