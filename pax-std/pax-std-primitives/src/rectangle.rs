use kurbo::{RoundedRect, Shape};
use piet::{LinearGradient, RadialGradient, RenderContext};
use std::any::Any;

use pax_core::{
    handle_vtable_update, with_properties_unwrapped, BaseInstance, ExpandedNode, InstanceFlags,
    InstanceNode, InstantiationArgs, PropertiesTreeContext, RenderTreeContext,
};
use pax_std::primitives::Rectangle;
use pax_std::types::Fill;

use pax_runtime_api::{Layer, Size};

use std::cell::RefCell;
use std::rc::Rc;

/// A basic 2D vector rectangle, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
pub struct RectangleInstance<R> {
    base: BaseInstance<R>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for RectangleInstance<R> {
    fn instantiate(args: InstantiationArgs<R>) -> Rc<Self> {
        Rc::new(Self {
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

    fn expand_node_and_compute_properties(
        &self,
        ptc: &mut PropertiesTreeContext<R>,
    ) -> Rc<RefCell<ExpandedNode<R>>> {
        let this_expanded_node = self.base().expand(ptc);
        let properties_wrapped = this_expanded_node.borrow().get_properties();

        with_properties_unwrapped!(
            &properties_wrapped,
            Rectangle,
            |properties: &mut Rectangle| {
                handle_vtable_update!(ptc, properties.stroke, pax_std::types::Stroke);
                handle_vtable_update!(ptc, properties.fill, pax_std::types::Fill);
                handle_vtable_update!(
                    ptc,
                    properties.corner_radii,
                    pax_std::types::RectangleCornerRadii
                );

                // TODO: figure out best practice for nested properties struct (perhaps higher-level struct is not Property<> wrapped?)
                // handle_vtable_update!(ptc, corner_radii.bottom_left, f64);
                // handle_vtable_update!(ptc, corner_radii.bottom_right, f64);
                // handle_vtable_update!(ptc, corner_radii.top_left, f64);
                // handle_vtable_update!(ptc, corner_radii.top_right, f64);
            }
        );

        this_expanded_node
    }

    fn get_clipping_size(&self, expanded_node: &ExpandedNode<R>) -> Option<(Size, Size)> {
        Some(self.get_size(expanded_node))
    }

    fn handle_render(&self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        let expanded_node = rtc.current_expanded_node.borrow();
        let tab = &expanded_node.computed_tab.as_ref().unwrap();

        let width: f64 = tab.bounds.0;
        let height: f64 = tab.bounds.1;

        let properties_wrapped: Rc<RefCell<dyn Any>> =
            rtc.current_expanded_node.borrow().get_properties();

        with_properties_unwrapped!(
            &properties_wrapped,
            Rectangle,
            |properties: &mut Rectangle| {
                let rect = RoundedRect::new(0.0, 0.0, width, height, properties.corner_radii.get());
                let bez_path = rect.to_path(0.1);

                let transformed_bez_path = tab.transform * bez_path;
                let duplicate_transformed_bez_path = transformed_bez_path.clone();

                match properties.fill.get() {
                    Fill::Solid(color) => {
                        rc.fill(transformed_bez_path, &color.to_piet_color());
                    }
                    Fill::LinearGradient(linear) => {
                        let linear_gradient = LinearGradient::new(
                            Fill::to_unit_point(linear.start, (width, height)),
                            Fill::to_unit_point(linear.end, (width, height)),
                            Fill::to_piet_gradient_stops(linear.stops.clone()),
                        );
                        rc.fill(transformed_bez_path, &linear_gradient)
                    }
                    Fill::RadialGradient(radial) => {
                        let origin = Fill::to_unit_point(radial.start, (width, height));
                        let center = Fill::to_unit_point(radial.end, (width, height));
                        let gradient_stops = Fill::to_piet_gradient_stops(radial.stops.clone());
                        let radial_gradient = RadialGradient::new(radial.radius, gradient_stops)
                            .with_center(center)
                            .with_origin(origin);
                        rc.fill(transformed_bez_path, &radial_gradient);
                    }
                }

                //hack to address "phantom stroke" bug on Web
                let width: f64 = *&properties.stroke.get().width.get().into();
                if width > f64::EPSILON {
                    rc.stroke(
                        duplicate_transformed_bez_path,
                        &properties.stroke.get().color.get().to_piet_color(),
                        width,
                    );
                }
            }
        );
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode<R>>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => {
                with_properties_unwrapped!(
                    &expanded_node.get_properties(),
                    Rectangle,
                    |r: &mut Rectangle| {
                        f.debug_struct("Rectangle")
                            .field("fill", r.fill.get())
                            .finish()
                    }
                )
            }
            None => f.debug_struct("Rectangle").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance<R> {
        &self.base
    }
}
