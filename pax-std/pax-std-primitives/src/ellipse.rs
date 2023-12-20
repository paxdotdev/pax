use kurbo::{Rect, Shape};
use pax_core::{declarative_macros::handle_vtable_update, BaseInstance};
use pax_runtime_api::{Layer, RenderContext};
use pax_std::{primitives::Ellipse, types::Fill};

use pax_core::{
    ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, PropertiesTreeContext,
    RenderTreeContext,
};

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

    fn expand(self: Rc<Self>, ptc: &mut PropertiesTreeContext) -> Rc<ExpandedNode> {
        let this_expanded_node = self
            .base()
            .expand_from_instance(Rc::clone(&self) as Rc<dyn InstanceNode>, ptc);

        this_expanded_node
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        _rtc: &RenderTreeContext,
        rc: &mut Box<dyn RenderContext>,
    ) {
        let computed_props = expanded_node.computed_expanded_properties.borrow();
        let tab = &computed_props.as_ref().unwrap().computed_tab;

        let width: f64 = tab.bounds.0;
        let height: f64 = tab.bounds.1;
        expanded_node.with_properties_unwrapped(|properties: &mut Ellipse| {
            let rect = Rect::from_points((0.0, 0.0), (width, height));
            let ellipse = kurbo::Ellipse::from_rect(rect);
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
            Some(expanded_node) => expanded_node
                .with_properties_unwrapped(|_e: &mut Ellipse| f.debug_struct("Ellipse").finish()),
            None => f.debug_struct("Ellipse").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn update(
        &self,
        expanded_node: &ExpandedNode,
        context: &pax_core::UpdateContext,
        messages: &mut Vec<pax_message::NativeMessage>,
    ) {
        expanded_node.with_properties_unwrapped(|properties: &mut Ellipse| {
            handle_vtable_update(
                context.expression_table,
                expanded_node,
                &mut properties.stroke,
            );
            handle_vtable_update(
                context.expression_table,
                expanded_node,
                &mut properties.fill,
            );
        });
    }
}
