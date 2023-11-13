use std::any::Any;
use kurbo::{RoundedRect, Shape};
use piet::{LinearGradient, RadialGradient, RenderContext};

use pax_core::{handle_vtable_update, HandlerRegistry, InstantiationArgs, PropertiesComputable, InstanceNode, InstanceNodePtr, InstanceNodePtrList, RenderTreeContext, ExpandedNode, PropertiesTreeContext};
use pax_std::primitives::Rectangle;
use pax_std::types::{Fill, RectangleCornerRadii};

use pax_runtime_api::CommonProperties;

use std::cell::RefCell;
use std::rc::Rc;

/// A basic 2D vector rectangle, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
pub struct RectangleInstance<R: 'static + RenderContext> {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub instance_id: u32,

    instance_prototypical_properties: Rc<RefCell<dyn Any>>,
    instance_prototypical_common_properties: Rc<RefCell<CommonProperties>>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for RectangleInstance<R> {

    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }

    fn get_instance_children(&self) -> InstanceNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized,
    {
        let mut node_registry = (*args.node_registry).borrow_mut();
        let instance_id = node_registry.mint_instance_id();
        let ret = Rc::new(RefCell::new(RectangleInstance {
            instance_id,
            handler_registry: args.handler_registry,
            instance_prototypical_common_properties: args.common_properties,
            instance_prototypical_properties: args.properties,
        }));

        node_registry.register(instance_id, Rc::clone(&ret) as InstanceNodePtr<R>);
        ret
    }

    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry<R>>>> {
        match &self.handler_registry {
            Some(registry) => Some(Rc::clone(registry)),
            _ => None,
        }
    }

    fn expand_node_and_compute_properties(&mut self, ptc: &mut PropertiesTreeContext<R>) -> Rc<RefCell<ExpandedNode<R>>> {
        let this_expanded_node = ExpandedNode::get_or_create_with_prototypical_properties(ptc, &self.instance_prototypical_properties, &self.instance_prototypical_common_properties);
        let properties_wrapped = this_expanded_node.borrow().get_properties();

        // with_properties_unsafe!(&properties_wrapped, PropertiesCoproduct, Rectangle, |properties : &mut Rectangle| {
        //     handle_vtable_update!(ptc, properties.stroke, pax_std::types::Stroke);
        //     handle_vtable_update!(ptc, properties.fill, pax_std::types::Fill);
        //     handle_vtable_update!(ptc, properties.corner_radii, pax_std::types::RectangleCornerRadii);
        //
        //     // handle_vtable_update!(ptc, corner_radii.bottom_left, f64);
        //     // handle_vtable_update!(ptc, corner_radii.bottom_right, f64);
        //     // handle_vtable_update!(ptc, corner_radii.top_left, f64);
        //     // handle_vtable_update!(ptc, corner_radii.top_right, f64);
        // });

        this_expanded_node
    }

    fn handle_render(&mut self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {

        let expanded_node = rtc.current_expanded_node.borrow();
        let tab = &expanded_node.tab;

        let width: f64 = tab.bounds.0;
        let height: f64 = tab.bounds.1;

        let properties_wrapped : Rc<RefCell<dyn Any>> = rtc.current_expanded_node.borrow().get_properties();
        // with_properties_unsafe!(&properties_wrapped, PropertiesCoproduct, Rectangle, |properties : &mut Rectangle|{
        //
        //     let rect = RoundedRect::new(0.0, 0.0, width, height, properties.corner_radii.get());
        //     let bez_path = rect.to_path(0.1);
        //
        //     let transformed_bez_path = tab.transform * bez_path;
        //     let duplicate_transformed_bez_path = transformed_bez_path.clone();
        //
        //     match properties.fill.get() {
        //         Fill::Solid(color) => {
        //             rc.fill(transformed_bez_path, &color.to_piet_color());
        //         }
        //         Fill::LinearGradient(linear) => {
        //             let linear_gradient = LinearGradient::new(
        //                 Fill::to_unit_point(linear.start, (width, height)),
        //                 Fill::to_unit_point(linear.end, (width, height)),
        //                 Fill::to_piet_gradient_stops(linear.stops.clone()),
        //             );
        //             rc.fill(transformed_bez_path, &linear_gradient)
        //         }
        //         Fill::RadialGradient(radial) => {
        //             let origin = Fill::to_unit_point(radial.start, (width, height));
        //             let center = Fill::to_unit_point(radial.end, (width, height));
        //             let gradient_stops = Fill::to_piet_gradient_stops(radial.stops.clone());
        //             let radial_gradient = RadialGradient::new(radial.radius, gradient_stops)
        //                 .with_center(center)
        //                 .with_origin(origin);
        //             rc.fill(transformed_bez_path, &radial_gradient);
        //         }
        //     }
        //
        //     //hack to address "phantom stroke" bug on Web
        //     let width: f64 = *&properties.stroke.get().width.get().into();
        //     if width > f64::EPSILON {
        //         rc.stroke(
        //             duplicate_transformed_bez_path,
        //             &properties.stroke.get().color.get().to_piet_color(),
        //             width,
        //         );
        //     }
        // });

        todo!()


        // let properties = (*self.properties).borrow();
        //
        // let rect = RoundedRect::new(0.0, 0.0, width, height, properties.corner_radii.get());
        //
        // let bez_path = rect.to_path(0.1);
        //
        // let transformed_bez_path = transform * bez_path;
        // let duplicate_transformed_bez_path = transformed_bez_path.clone();
        //
        //
        //
        // //hack to address "phantom stroke" bug on Web
        // let width: f64 = *&properties.stroke.get().width.get().into();
        // if width > f64::EPSILON {
        //     rc.stroke(
        //         duplicate_transformed_bez_path,
        //         &properties.stroke.get().color.get().to_piet_color(),
        //         width,
        //     );
        // }
    }
}
