use kurbo::BezPath;

use pax_runtime::api::{Layer, RenderContext};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use pax_runtime_api::{borrow, borrow_mut, use_RefCell};
use pax_std::primitives::Path;
use pax_std::types::path_types::PathContext;
use pax_std::types::{PathElement, Point};

use_RefCell!();
use std::collections::HashMap;
use std::iter;
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

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        // create a new stack to be able to insert a local store specific for this node and the
        // ones bellow. If not done, things above this node could potentially access it
        let env = expanded_node
            .stack
            .push(HashMap::new(), &*borrow!(expanded_node.properties));
        expanded_node.with_properties_unwrapped(|properties: &mut Path| {
            env.insert_stack_local_store(PathContext {
                elements: properties.elements.clone(),
            });
            let children = borrow!(self.base().get_instance_children());
            let children_with_envs = children.iter().cloned().zip(iter::repeat(env));
            let new_children = expanded_node.generate_children(children_with_envs, context);
            // set slot children to all to make children compute and update their slot index
            // (see expanded_node compute_expanded and flattened children)
            *borrow_mut!(expanded_node.expanded_slot_children) = Some(new_children.clone());
            expanded_node.children.set(new_children);
        });
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {
        // NOTE: do not update children here,
        // we know that all of the expanded and flattened children
        // are the same as the once being rendered
        expanded_node.compute_flattened_slot_children();
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        _rtc: &Rc<RuntimeContext>,
        rc: &mut dyn RenderContext,
    ) {
        let layer_id = format!("{}", borrow!(expanded_node.occlusion).0);

        expanded_node.with_properties_unwrapped(|properties: &mut Path| {
            let mut bez_path = BezPath::new();

            let bounds = expanded_node.transform_and_bounds.get().bounds;

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

            let tab = expanded_node.transform_and_bounds.get();
            let transformed_bez_path = Into::<kurbo::Affine>::into(tab.transform) * bez_path;
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
                    properties
                        .stroke
                        .get()
                        .width
                        .get()
                        .expect_pixels()
                        .to_float(),
                );
            }
        });
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Path").finish()
    }
}
