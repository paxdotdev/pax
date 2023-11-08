use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_core::{
    unsafe_unwrap, HandlerRegistry, InstantiationArgs, PropertiesComputable, RenderNode,
    RenderNodePtr, RenderNodePtrList, RenderTreeContext,
};
use pax_pixels::{Angle, Color, Fill, Point2D, RenderContext, Stroke, Vector2D, Winding};
use pax_std::primitives::Ellipse;
use pax_std::types::ColorVariant;

use pax_runtime_api::CommonProperties;

use std::cell::RefCell;
use std::rc::Rc;

/// A basic 2D vector ellipse, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
pub struct EllipseInstance<R: 'static + RenderContext> {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub instance_id: u32,
    pub properties: Rc<RefCell<Ellipse>>,
    pub common_properties: CommonProperties,
}

impl<R: 'static + RenderContext> RenderNode<R> for EllipseInstance<R> {
    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }

    fn get_common_properties(&self) -> &CommonProperties {
        &self.common_properties
    }

    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized,
    {
        let properties = unsafe_unwrap!(args.properties, PropertiesCoproduct, Ellipse);
        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(EllipseInstance {
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
        self.common_properties.compute_properties(rtc);

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
            let new_value = unsafe_unwrap!(fill, TypesCoproduct, pax_std::types::Color);
            properties.fill.set(new_value);
        }
    }
    fn handle_render(&mut self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        let transform = rtc.transform_scroller_reset;
        let bounding_dimens = rtc.bounds;
        let width: f64 = bounding_dimens.0;
        let height: f64 = bounding_dimens.1;

        let properties = (*self.properties).borrow();

        let properties_color = properties.fill.get();
        let _color = match properties_color.color_variant {
            ColorVariant::Hlca(slice) => Color::hlca(
                slice[0] as f32,
                slice[1] as f32,
                slice[2] as f32,
                slice[3] as f32,
            ),
            ColorVariant::Hlc(slice) => {
                Color::hlca(slice[0] as f32, slice[1] as f32, slice[2] as f32, 1.0)
            }
            ColorVariant::Rgba(slice) => Color::rgba(
                slice[0] as f32,
                slice[1] as f32,
                slice[2] as f32,
                slice[3] as f32,
            ),
            ColorVariant::Rgb(slice) => {
                Color::rgba(slice[0] as f32, slice[1] as f32, slice[2] as f32, 1.0)
            }
        };

        let mut builder = pax_pixels::Path::builder().transformed(transform);
        builder.add_ellipse(
            Point2D::new(0.0, 0.0),
            Vector2D::new(width as f32, height as f32),
            Angle::degrees(0.0),
            Winding::Positive,
        );

        let path = builder.build();

        let color = properties.fill.get().to_pax_pixels_color();
        rc.fill_path(path.clone(), Fill::Solid(color));

        //hack to address "phantom stroke" bug on Web
        let weight: f64 = properties.stroke.get().width.get().into();
        if weight > f64::EPSILON {
            rc.stroke_path(
                path,
                Stroke {
                    color: properties.stroke.get().color.get().to_pax_pixels_color(),
                    weight: weight as f32,
                },
            );
        }
    }
}
