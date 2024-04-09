use kurbo::BezPath;

use pax_runtime::api::{Layer, RenderContext};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use pax_std::primitives::Path;
use pax_std::types::{PathElement, Point};

use std::cell::RefCell;
use std::rc::Rc;

/// A basic 2D vector path for arbitrary Bézier / line-segment chains
pub struct PathInstance {
    base: BaseInstance,
}

impl InstanceNode for PathInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: Layer::Canvas,
                    is_component: false,
                },
            ),
        })
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        _rtc: &Rc<RefCell<RuntimeContext>>,
        rc: &mut dyn RenderContext,
    ) {
        let layer_id = format!("{}", expanded_node.occlusion_id.borrow());

        expanded_node.with_properties_unwrapped(|properties: &mut Path| {
            let mut bez_path = BezPath::new();

            let bounds = expanded_node.layout_properties.bounds.get();

            let elems = properties.elements.get();
            let mut itr_elems = elems.iter();

            if let Some(elem) = itr_elems.next() {
                if let &PathElement::Point(x, y) = elem {
                    bez_path.move_to(Point { x, y }.to_kurbo_point(bounds));
                } else {
                    log::warn!("path must start with point");
                    return;
                }
            }

            while let Some(elem) = itr_elems.next() {
                match elem {
                    &PathElement::Point(x, y) => {
                        bez_path.move_to(Point { x, y }.to_kurbo_point(bounds));
                    }
                    &PathElement::Line => {
                        let Some(&PathElement::Point(x, y)) = itr_elems.next() else {
                            log::warn!("line expects to be followed by a point");
                            return;
                        };
                        bez_path.line_to(Point { x, y }.to_kurbo_point(bounds));
                    }
                    &PathElement::Curve(h_x, h_y) => {
                        let Some(&PathElement::Point(x, y)) = itr_elems.next() else {
                            log::warn!("curve expects to be followed by a point");
                            return;
                        };
                        bez_path.quad_to(
                            Point { x: h_x, y: h_y }.to_kurbo_point(bounds),
                            Point { x, y }.to_kurbo_point(bounds),
                        );
                    }
                    &PathElement::Close => {
                        bez_path.close_path();
                    }
                    PathElement::Empty => (), //no-op
                }
            }

            let tab = &expanded_node.layout_properties;
            let transformed_bez_path = Into::<kurbo::Affine>::into(tab.transform.get()) * bez_path;
            let duplicate_transformed_bez_path = transformed_bez_path.clone();

            let color = properties.fill.get().to_piet_color();
            rc.fill(&layer_id, transformed_bez_path, &color.into());
            if properties
                .stroke
                .get()
                .width
                .get()
                .expect_pixels()
                .to_float()
                > f64::EPSILON
            {
                rc.stroke(
                    &layer_id,
                    duplicate_transformed_bez_path,
                    &properties.stroke.get().color.get().to_piet_color().into(),
                    properties.stroke.get().width.get().expect_pixels().into(),
                );
            }
        });
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Path").finish()
    }
}
