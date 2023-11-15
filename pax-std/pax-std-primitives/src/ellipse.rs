use kurbo::{Ellipse as KurboEllipse, Rect, Shape};
use piet::RenderContext;
use std::any::Any;

use pax_core::{
    handle_vtable_update, with_properties_unwrapped, ExpandedNode, HandlerRegistry, InstanceNode,
    InstanceNodePtr, InstanceNodePtrList, InstantiationArgs, PropertiesTreeContext,
    RenderTreeContext,
};

use pax_std::primitives::Ellipse;
use pax_std::types::Fill;

use pax_runtime_api::CommonProperties;

use std::cell::RefCell;
use std::rc::Rc;

/// A basic 2D vector ellipse, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
pub struct EllipseInstance {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
    pub instance_id: u32,
    instance_prototypical_properties: Rc<RefCell<dyn Any>>,
    instance_prototypical_common_properties: Rc<RefCell<CommonProperties>>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for EllipseInstance {
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
        let ret = Rc::new(RefCell::new(EllipseInstance {
            instance_id,
            handler_registry: args.handler_registry,
            instance_prototypical_common_properties: args.common_properties,
            instance_prototypical_properties: args.properties,
        }));

        node_registry.register(instance_id, Rc::clone(&ret) as InstanceNodePtr<R>);
        ret
    }

    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry>>> {
        match &self.handler_registry {
            Some(registry) => Some(Rc::clone(registry)),
            _ => None,
        }
    }
    fn expand_node_and_compute_properties(
        &mut self,
        ptc: &mut PropertiesTreeContext<R>,
    ) -> Rc<RefCell<ExpandedNode<R>>> {
        let this_expanded_node = ExpandedNode::get_or_create_with_prototypical_properties(
            ptc,
            &self.instance_prototypical_properties,
            &self.instance_prototypical_common_properties,
        );
        let properties_wrapped = this_expanded_node.borrow().get_properties();

        with_properties_unwrapped!(&properties_wrapped, Ellipse, |properties: &mut Ellipse| {
            handle_vtable_update!(ptc, properties.stroke, pax_std::types::Stroke);
            handle_vtable_update!(ptc, properties.fill, pax_std::types::Fill);
        });

        this_expanded_node
    }
    fn handle_render(&mut self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        let expanded_node = rtc.current_expanded_node.borrow();
        let tab = expanded_node.computed_tab.as_ref().unwrap();

        let width: f64 = tab.bounds.0;
        let height: f64 = tab.bounds.1;
        let properties_wrapped: Rc<RefCell<dyn Any>> =
            rtc.current_expanded_node.borrow().get_properties();
        with_properties_unwrapped!(&properties_wrapped, Ellipse, |properties: &mut Ellipse| {
            let rect = Rect::from_points((0.0, 0.0), (width, height));
            let ellipse = KurboEllipse::from_rect(rect);
            let accuracy = 0.1;
            let bez_path = ellipse.to_path(accuracy);

            let transformed_bez_path = tab.transform * bez_path;
            let duplicate_transformed_bez_path = transformed_bez_path.clone();

            let color = if let Fill::Solid(properties_color) = properties.fill.get() {
                properties_color.to_piet_color()
            } else {
                unimplemented!("gradients not supported on ellipse")
            };

            rc.fill(transformed_bez_path, &color);

            //hack to address "phantom stroke" bug on Web
            let width: f64 = *&properties.stroke.get().width.get().into();
            if width > f64::EPSILON {
                rc.stroke(
                    duplicate_transformed_bez_path,
                    &properties.stroke.get().color.get().to_piet_color(),
                    width,
                );
            }
        });
    }
}
