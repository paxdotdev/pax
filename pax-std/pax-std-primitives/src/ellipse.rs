use kurbo::{Ellipse as KurboEllipse, Rect, Shape};
use pax_runtime_api::{Layer, RenderContext};
use std::any::Any;

use pax_core::{
    handle_vtable_update, with_properties_unwrapped, BaseInstance, ExpandedNode, InstanceFlags,
    InstanceNode, InstantiationArgs, PropertiesTreeContext, RenderTreeContext,
};

use pax_std::primitives::Ellipse;
use pax_std::types::Fill;

use std::cell::RefCell;
use std::rc::Rc;

/// A basic 2D vector ellipse, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
pub struct EllipseInstance {
    base: BaseInstance,
}

impl InstanceNode for EllipseInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(EllipseInstance {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    layer: Layer::Canvas,
                },
            ),
        })
    }

    fn expand(self: Rc<Self>, ptc: &mut PropertiesTreeContext) -> Rc<RefCell<ExpandedNode>> {
        let this_expanded_node = self
            .base()
            .expand(Rc::clone(&self) as Rc<dyn InstanceNode>, ptc);
        let properties_wrapped = this_expanded_node.borrow().get_properties();

        with_properties_unwrapped!(&properties_wrapped, Ellipse, |properties: &mut Ellipse| {
            handle_vtable_update!(
                ptc,
                this_expanded_node,
                properties.stroke,
                pax_std::types::Stroke
            );
            handle_vtable_update!(
                ptc,
                this_expanded_node,
                properties.fill,
                pax_std::types::Fill
            );
        });

        this_expanded_node
    }

    fn handle_render(&self, rtc: &mut RenderTreeContext, rc: &mut Box<dyn RenderContext>) {
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

            rc.fill(transformed_bez_path, &color.into());

            //hack to address "phantom stroke" bug on Web
            let width: f64 = *&properties.stroke.get().width.get().into();
            if width > f64::EPSILON {
                rc.stroke(
                    duplicate_transformed_bez_path,
                    &properties.stroke.get().color.get().to_piet_color().into(),
                    width,
                );
            }
        });
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => {
                with_properties_unwrapped!(
                    &expanded_node.get_properties(),
                    Ellipse,
                    |_e: &mut Ellipse| { f.debug_struct("Ellipse").finish() }
                )
            }
            None => f.debug_struct("Ellipse").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
