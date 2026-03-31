use kurbo::{Affine, Rect, Shape};
use pax_engine::*;
use pax_runtime::api::{use_RefCell, Stroke};
use pax_runtime::api::{Fill, Layer, RenderContext};
use pax_runtime::BaseInstance;
use pax_runtime::{ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext};
use_RefCell!();
use std::rc::Rc;

const ELLIPSE_PATH_ACCURACY: f64 = 0.01;

/// A basic 2D vector ellipse
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::ellipse::EllipseInstance")]
pub struct Ellipse {
    pub stroke: Property<Stroke>,
    pub fill: Property<Fill>,
}

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
                    is_slot: false,
                },
            ),
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let tab = expanded_node.transform_and_bounds.clone();
        let (stroke, fill) = expanded_node.with_properties_unwrapped(|properties: &mut Ellipse| {
            (properties.stroke.clone(), properties.fill.clone())
        });

        let deps = &[
            tab.untyped(),
            stroke.untyped(),
            fill.untyped(),
            expanded_node.computed_opacity.untyped(),
        ];
        let cloned_expanded_node = expanded_node.clone();
        let cloned_context = context.clone();

        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    cloned_context.mark_canvas_node_dirty(cloned_expanded_node.id);
                    cloned_context
                        .set_canvas_dirty(cloned_expanded_node.occlusion.get().occlusion_layer_id)
                },
                deps,
            ));
    }

    fn update(self: Rc<Self>, _expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {}

    fn resolve_coverage_path(&self, expanded_node: &ExpandedNode) -> Option<kurbo::BezPath> {
        let tab = expanded_node.transform_and_bounds.get();
        let (width, height) = tab.bounds;
        let rect = Rect::from_points((0.0, 0.0), (width, height));
        let ellipse = kurbo::Ellipse::from_rect(rect);
        Some(Affine::from(tab.transform) * ellipse.to_path(ELLIPSE_PATH_ACCURACY))
    }

    fn resolve_coverage_opacity(&self, expanded_node: &ExpandedNode) -> f64 {
        expanded_node.with_properties_unwrapped(|properties: &mut Ellipse| {
            (properties.fill.get().max_alpha_0_1() * expanded_node.computed_opacity.get())
                .clamp(0.0, 1.0)
        })
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rc: &mut dyn RenderContext,
    ) {
        let layer_id = expanded_node.occlusion.get().occlusion_layer_id;

        if !rtc.is_canvas_node_dirty(&expanded_node.id) {
            return;
        }

        if !rc.begin_node(
            layer_id,
            expanded_node.id.to_u32(),
            expanded_node.occlusion.get().z_index,
        ) {
            return;
        }

        let tab = expanded_node.transform_and_bounds.get();
        let (width, height) = tab.bounds;
        expanded_node.with_properties_unwrapped(|properties: &mut Ellipse| {
            let rect = Rect::from_points((0.0, 0.0), (width, height));
            let ellipse = kurbo::Ellipse::from_rect(rect);
            let bez_path = ellipse.to_path(ELLIPSE_PATH_ACCURACY);
            let opacity = expanded_node.computed_opacity.get();
            let fill = properties.fill.get();
            let stroke_color = properties.stroke.get().color.get();
            rc.save(layer_id);
            rc.transform(layer_id, tab.transform.into());
            rc.fill_with_opacity(layer_id, bez_path.clone(), &fill, opacity);

            //hack to address "phantom stroke" bug on Web
            let width: f64 = properties
                .stroke
                .get()
                .width
                .get()
                .expect_pixels()
                .to_float();

            if width > f64::EPSILON {
                rc.stroke_with_opacity(
                    layer_id,
                    bez_path,
                    &Fill::Solid(stroke_color),
                    width,
                    opacity,
                );
            }
            rc.restore(layer_id);
        });
        if rc.end_node(layer_id, expanded_node.id.to_u32()) {
            rtc.clear_canvas_node_dirty(&expanded_node.id);
        }
    }

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
}
