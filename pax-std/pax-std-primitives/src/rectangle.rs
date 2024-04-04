use kurbo::{RoundedRect, Shape};
use pax_runtime::{declarative_macros::handle_vtable_update, BaseInstance};
use piet::{LinearGradient, RadialGradient};

use pax_runtime::{ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext};
use pax_std::primitives::Rectangle;
use pax_std::types::Fill;

use pax_runtime::api::{Layer, RenderContext, Size};

use std::{cell::RefCell, rc::Rc};

/// A basic 2D vector rectangle, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
pub struct RectangleInstance {
    base: BaseInstance,
}

impl InstanceNode for RectangleInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
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

    fn update(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        // //Doesn't need to expand any children
        // expanded_node.with_properties_unwrapped(|properties: &mut Rectangle| {
        //     handle_vtable_update(
        //         &context.borrow().expression_table(),
        //         &expanded_node.stack,
        //         &mut properties.stroke,
        //         context.borrow().globals(),
        //     );
        //     handle_vtable_update(
        //         &context.borrow().expression_table(),
        //         &expanded_node.stack,
        //         &mut properties.stroke.get().color,
        //         context.borrow().globals(),
        //     );
        //     handle_vtable_update(
        //         &context.borrow().expression_table(),
        //         &expanded_node.stack,
        //         &mut properties.stroke.get().width,
        //         context.borrow().globals(),
        //     );
        //     handle_vtable_update(
        //         &context.borrow().expression_table(),
        //         &expanded_node.stack,
        //         &mut properties.fill,
        //         context.borrow().globals(),
        //     );
        //     handle_vtable_update(
        //         &context.borrow().expression_table(),
        //         &expanded_node.stack,
        //         &mut properties.corner_radii,
        //         context.borrow().globals(),
        //     );

        //     // TODO: figure out best practice for nested properties struct (perhaps higher-level struct is not Property<> wrapped?)
        //     // handle_vtable_update!(ptc, corner_radii.bottom_left, f64);
        //     // handle_vtable_update!(ptc, corner_radii.bottom_right, f64);
        //     // handle_vtable_update!(ptc, corner_radii.top_left, f64);
        //     // handle_vtable_update!(ptc, corner_radii.top_right, f64);
        // });
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        _rtc: &Rc<RefCell<RuntimeContext>>,
        rc: &mut dyn RenderContext,
    ) {
        let tab = &expanded_node.layout_properties;
        let (width, height) = tab.bounds.get();

        let layer_id = format!("{}", expanded_node.occlusion_id.borrow());

        expanded_node.with_properties_unwrapped(|properties: &mut Rectangle| {
            let rect = RoundedRect::new(0.0, 0.0, width, height, &properties.corner_radii.get());
            let bez_path = rect.to_path(0.1);

            let transformed_bez_path = Into::<kurbo::Affine>::into(tab.transform.get()) * bez_path;
            let duplicate_transformed_bez_path = transformed_bez_path.clone();

            match properties.fill.get() {
                Fill::Solid(color) => {
                    rc.fill(
                        &layer_id,
                        transformed_bez_path,
                        &color.to_piet_color().into(),
                    );
                }
                Fill::LinearGradient(linear) => {
                    let linear_gradient = LinearGradient::new(
                        Fill::to_unit_point(linear.start, (width, height)),
                        Fill::to_unit_point(linear.end, (width, height)),
                        Fill::to_piet_gradient_stops(linear.stops.clone()),
                    );
                    rc.fill(&layer_id, transformed_bez_path, &linear_gradient.into())
                }
                Fill::RadialGradient(radial) => {
                    let origin = Fill::to_unit_point(radial.start, (width, height));
                    let center = Fill::to_unit_point(radial.end, (width, height));
                    let gradient_stops = Fill::to_piet_gradient_stops(radial.stops.clone());
                    let radial_gradient = RadialGradient::new(radial.radius, gradient_stops)
                        .with_center(center)
                        .with_origin(origin);
                    rc.fill(&layer_id, transformed_bez_path, &radial_gradient.into());
                }
            }

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
            Some(expanded_node) => expanded_node.with_properties_unwrapped(|r: &mut Rectangle| {
                f.debug_struct("Rectangle")
                    .field("fill", &r.fill.get())
                    .finish()
            }),
            None => f.debug_struct("Rectangle").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
