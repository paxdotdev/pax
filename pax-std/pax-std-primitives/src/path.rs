use kurbo::BezPath;
use pax_runtime::declarative_macros::handle_vtable_update;

use pax_runtime::api::{Layer, RenderContext};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use pax_std::primitives::Path;
use pax_std::types::PathSegment;

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

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &mut RuntimeContext) {
        expanded_node.with_properties_unwrapped(|properties: &mut Path| {
            let tbl = context.expression_table();
            let stk = &expanded_node.stack;
            handle_vtable_update(tbl, stk, &mut properties.stroke, context.globals());
            handle_vtable_update(
                tbl,
                stk,
                &mut properties.stroke.get_mut().color,
                context.globals(),
            );
            handle_vtable_update(tbl, stk, &mut properties.fill, context.globals());
            handle_vtable_update(tbl, stk, &mut properties.segments, context.globals());
        });
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        _rtc: &mut RuntimeContext,
        rc: &mut dyn RenderContext,
    ) {
        let layer_id = format!("{}", expanded_node.occlusion_id.borrow());

        expanded_node.with_properties_unwrapped(|properties: &mut Path| {
            let mut bez_path = BezPath::new();

            let layout_props = expanded_node.layout_properties.borrow();
            let bounds = layout_props.as_ref().unwrap().computed_tab.bounds;

            for segment in properties.segments.get().iter() {
                match segment {
                    PathSegment::Empty => { /* no-op */ }
                    PathSegment::LineSegment(data) => {
                        bez_path.move_to(data.start.to_kurbo_point(bounds));
                        bez_path.line_to(data.end.to_kurbo_point(bounds));
                    }
                    PathSegment::CurveSegment(data) => {
                        bez_path.move_to(data.start.to_kurbo_point(bounds));
                        bez_path.quad_to(
                            data.handle.to_kurbo_point(bounds),
                            data.end.to_kurbo_point(bounds),
                        );
                    }
                }
            }

            let computed_props = expanded_node.layout_properties.borrow();
            let tab = &computed_props.as_ref().unwrap().computed_tab;

            let transformed_bez_path = Into::<kurbo::Affine>::into(tab.transform) * bez_path;
            let duplicate_transformed_bez_path = transformed_bez_path.clone();

            let color = properties.fill.get().to_piet_color();
            rc.fill(&layer_id, transformed_bez_path, &color.into());
            rc.stroke(
                &layer_id,
                duplicate_transformed_bez_path,
                &properties.stroke.get().color.get().to_piet_color().into(),
                properties.stroke.get().width.get().expect_pixels().into(),
            );
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
