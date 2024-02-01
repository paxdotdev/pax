use kurbo::{Rect, Shape};
use pax_core::{declarative_macros::handle_vtable_update, BaseInstance};
use pax_runtime_api::{Layer, RenderContext};
use pax_std::{primitives::Ellipse, types::Fill};

use pax_core::{ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext};

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
                    is_component: false,
                },
            ),
        })
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        _context: &mut RuntimeContext,
        rc: &mut dyn RenderContext,
    ) {
        let computed_props = expanded_node.layout_properties.borrow();
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

            let layer_id = format!("{}", expanded_node.occlusion_id.borrow());
            rc.fill(&layer_id, transformed_bez_path, &color.into());

            //hack to address "phantom stroke" bug on Web
            let width: f64 = *&properties.stroke.get().width.get().expect_pixels().into();
            if width > f64::EPSILON {
                rc.stroke(
                    &layer_id,
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

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &mut RuntimeContext) {
        //Doesn't need to expand any children
        expanded_node.with_properties_unwrapped(|properties: &mut Ellipse| {
            handle_vtable_update(
                context.expression_table(),
                &expanded_node.stack,
                &mut properties.stroke,
            );
            handle_vtable_update(
                context.expression_table(),
                &expanded_node.stack,
                &mut properties.fill,
            );
        });
    }
}
