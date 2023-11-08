use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_core::{
    unsafe_unwrap, HandlerRegistry, InstantiationArgs, PropertiesComputable, RenderNode,
    RenderNodePtr, RenderNodePtrList, RenderTreeContext,
};
use pax_std::primitives::Rectangle;
use pax_std::types::{Fill, RectangleCornerRadii};

use pax_pixels::{Box2D, Point2D, RenderContext, Vector2D, Winding};
use pax_runtime_api::{log, CommonProperties};
use std::cell::RefCell;
use std::rc::Rc;

/// A basic 2D vector rectangle, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
pub struct RectangleInstance<R: 'static + RenderContext> {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub instance_id: u32,
    pub properties: Rc<RefCell<Rectangle>>,
    pub common_properties: CommonProperties,
}

impl<R: 'static + RenderContext> RenderNode<R> for RectangleInstance<R> {
    fn get_common_properties(&self) -> &CommonProperties {
        &self.common_properties
    }

    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }

    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized,
    {
        let properties = unsafe_unwrap!(args.properties, PropertiesCoproduct, Rectangle);
        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(RectangleInstance {
            instance_id,
            properties: Rc::new(RefCell::new(properties)),
            handler_registry: args.handler_registry,
            common_properties: args.common_properties,
        }));

        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }

    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry<R>>>> {
        match &self.handler_registry {
            Some(registry) => Some(Rc::clone(registry)),
            _ => None,
        }
    }

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {
        let properties = &mut *self.properties.as_ref().borrow_mut();

        if let Some(stroke_width) =
            rtc.compute_vtable_value(properties.stroke.get().width._get_vtable_id())
        {
            let new_value = if let TypesCoproduct::SizePixels(v) = stroke_width {
                v
            } else {
                unreachable!()
            };
            properties.stroke.get_mut().width.set(new_value);
        }

        if let Some(stroke_color) =
            rtc.compute_vtable_value(properties.stroke.get().color._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(stroke_color, TypesCoproduct, pax_std::types::Color);
            properties.stroke.get_mut().color.set(new_value);
        }

        if let Some(fill) = rtc.compute_vtable_value(properties.fill._get_vtable_id()) {
            let new_value = unsafe_unwrap!(fill, TypesCoproduct, Fill);
            properties.fill.set(new_value);
        }

        if let Some(top_right) =
            rtc.compute_vtable_value(properties.corner_radii.get().top_right._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(top_right, TypesCoproduct, f64);
            properties.corner_radii.get_mut().top_right.set(new_value);
        }

        if let Some(top_left) =
            rtc.compute_vtable_value(properties.corner_radii.get().top_left._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(top_left, TypesCoproduct, f64);
            properties.corner_radii.get_mut().top_left.set(new_value);
        }

        if let Some(bottom_right) =
            rtc.compute_vtable_value(properties.corner_radii.get().bottom_right._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(bottom_right, TypesCoproduct, f64);
            properties
                .corner_radii
                .get_mut()
                .bottom_right
                .set(new_value);
        }

        if let Some(bottom_left) =
            rtc.compute_vtable_value(properties.corner_radii.get().bottom_left._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(bottom_left, TypesCoproduct, f64);
            properties.corner_radii.get_mut().bottom_left.set(new_value);
        }

        if let Some(corner_radii) =
            rtc.compute_vtable_value(properties.corner_radii._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(corner_radii, TypesCoproduct, RectangleCornerRadii);
            properties.corner_radii.set(new_value);
        }

        self.common_properties.compute_properties(rtc);
    }

    fn handle_render(&mut self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        let transform = rtc.transform_scroller_reset;
        let bounding_dimens = rtc.bounds;
        let width: f64 = bounding_dimens.0;
        let height: f64 = bounding_dimens.1;

        let properties = (*self.properties).borrow();

        let rr = properties.corner_radii.get();
        let border_radii = pax_pixels::BorderRadii {
            top_left: *rr.top_left.get() as f32,
            top_right: *rr.top_right.get() as f32,
            bottom_left: *rr.bottom_left.get() as f32,
            bottom_right: *rr.bottom_right.get() as f32,
        };

        //TODOrefactor -
        // handle radial gradients
        let fill = match properties.fill.get() {
            Fill::Solid(color) => pax_pixels::Fill::Solid(color.to_pax_pixels_color()),
            Fill::LinearGradient(gradient) => {
                let x = gradient.start.0.get_pixels(width);
                let y = gradient.start.1.get_pixels(height);
                let x_e = gradient.end.0.get_pixels(width);
                let y_e = gradient.end.1.get_pixels(height);
                let pos = Point2D::new(x as f32, y as f32);
                let vec = Point2D::new(x_e as f32, y_e as f32) - pos;
                pax_pixels::Fill::Gradient {
                    gradient_type: pax_pixels::GradientType::Linear,
                    pos: transform.transform_point(pos),
                    main_axis: transform.transform_vector(vec),
                    off_axis: Vector2D::zero(),
                    stops: gradient
                        .stops
                        .iter()
                        .map(|s| pax_pixels::GradientStop {
                            color: s.color.to_pax_pixels_color(),
                            stop: (s.position.get_pixels(vec.length() as f64) / vec.length() as f64)
                                as f32,
                        })
                        .collect(),
                }
            }
            _ => {
                pax_runtime_api::log("radial gradients not supported yet");
                return;
            }
        };

        let mut builder = pax_pixels::Path::builder().transformed(transform);
        builder.add_rounded_rectangle(
            &Box2D::new(
                Point2D::new(0.0, 0.0),
                Point2D::new(width as f32, height as f32),
            ),
            &border_radii,
            Winding::Positive,
        );

        let path = builder.build();

        rc.fill_path(path.clone(), fill);

        //hack to address "phantom stroke" bug on Web
        let width: f64 = properties.stroke.get().width.get().into();
        if width > f64::EPSILON {
            rc.stroke_path(
                path,
                pax_pixels::Stroke {
                    color: properties.stroke.get().color.get().to_pax_pixels_color(),
                    weight: width as f32,
                },
            );
        }
    }
}
