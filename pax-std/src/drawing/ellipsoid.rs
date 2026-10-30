use pax_engine::{
    api::{Color, Depth, Fill, Layer, LocalProperty, Material, Property, RenderContext},
    pax,
};
use pax_runtime::api as pax_runtime_api;
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use pax_runtime_api::{
    math::{Generic, Point2, Transform3},
    mesh3::{ConvexMesh, OpaqueMeshDraw, PosedConvexMesh},
    use_RefCell, Window,
};
use std::rc::Rc;
use_RefCell!();

/// An opaque ellipsoid with width × height × depth extents, centered on local Z=0.
/// Requires `pax-kit/3d` and the GPU renderer. Ordinary transforms, hard clips,
/// lighting, and depth-aware native-control occlusion apply to the complete volume.
///
/// This initial mesh primitive accepts a solid opaque color. Fractional opacity,
/// alpha masks, detached mask sources, and shader-effect captures are unsupported;
/// unsupported instances are omitted with a diagnostic. Zero/negative dimensions
/// or a singular pose produce no coverage. Transparent shells are a separate contract.
#[pax]
#[custom(Default)]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::ellipsoid::EllipsoidInstance")]
pub struct Ellipsoid {
    /// Absolute local Z extent; defaults to 100 logical pixels.
    pub depth: Property<Depth>,
    /// Opaque surface color. Defaults to white.
    pub color: Property<Color>,
    /// Surface response to ordinary Pax scene lights.
    pub material: Property<Material>,
}
impl Default for Ellipsoid {
    fn default() -> Self {
        Self {
            depth: Property::new(Depth::from(100.)),
            color: Property::new(Color::WHITE),
            material: Property::new(Material::default()),
        }
    }
}
thread_local! { static SPHERE: Rc<ConvexMesh> = ConvexMesh::sphere(64,32); }
type Matrix = Transform3<Generic>;

pub struct EllipsoidInstance {
    base: BaseInstance,
    gpu: std::cell::Cell<bool>,
}

fn supported(node: &ExpandedNode) -> bool {
    if node.is_render_source() || node.computed_opacity.get() != 1. {
        return false;
    }
    let opaque = node.with_properties_unwrapped(|p: &mut Ellipsoid| {
        p.color.local().get().to_rgba_0_1()[3] == 1.
    });
    if !opaque {
        return false;
    }
    if !node.permits_opaque_mesh_content() {
        return false;
    }
    true
}
fn polygon_path(points: &[[f64; 2]]) -> kurbo::BezPath {
    let mut p = kurbo::BezPath::new();
    if let Some(first) = points.first() {
        p.move_to((first[0], first[1]));
        for v in &points[1..] {
            p.line_to((v[0], v[1]));
        }
        p.close_path();
    }
    p
}
impl InstanceNode for EllipsoidInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
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
            gpu: std::cell::Cell::new(false),
        })
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Ellipsoid").finish()
    }
    fn base(&self) -> &BaseInstance {
        &self.base
    }
    fn handle_mount(self: Rc<Self>, node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        self.gpu.set(context.globals().renderer_is_gpu);
        let (depth, color, material) = node.with_properties_unwrapped(|p: &mut Ellipsoid| {
            (p.depth.local(), p.color.local(), p.material.clone())
        });
        // Reuse the nested material subscription contract of ordinary paints.
        let appearance = crate::common::watch_appearance(
            node,
            context,
            LocalProperty::new(vec![Fill {
                material,
                ..Default::default()
            }]),
            LocalProperty::new(Vec::new()),
        );
        let deps = [
            depth.untyped(),
            color.untyped(),
            appearance.untyped(),
            node.spatial_geometry().untyped(),
            node.transform_and_bounds.untyped(),
            node.computed_opacity.untyped(),
            node.computed_opacity_scopes.untyped(),
        ];
        let n = node.clone();
        let ctx = context.clone();
        node.changed_listener.replace_with(LocalProperty::computed(
            move || {
                appearance.get();
                ctx.mark_canvas_node_dirty(n.id);
                ctx.mark_node_occlusion_dirty(n.id.to_u32());
                ctx.set_canvas_dirty(n.occlusion.get().render_layer_id);
            },
            &deps,
        ));
    }
    fn resolve_convex_mesh(&self, node: &ExpandedNode) -> Option<Rc<PosedConvexMesh>> {
        if !self.gpu.get() || !supported(node) {
            return None;
        }
        let (w, h) = node.transform_and_bounds.get().bounds;
        let d =
            node.with_properties_unwrapped(|p: &mut Ellipsoid| p.depth.local().get().to_float());
        if ![w, h, d].iter().all(|v| v.is_finite() && *v > 0.) {
            return None;
        }
        let local = Matrix::new([
            w / 2.,
            0.,
            0.,
            0.,
            0.,
            h / 2.,
            0.,
            0.,
            0.,
            0.,
            d / 2.,
            0.,
            w / 2.,
            h / 2.,
            0.,
            1.,
        ]);
        let world = Matrix::new(node.spatial_geometry().get().world_transform().coeffs()) * local;
        SPHERE.with(|mesh| mesh.posed(world.coeffs()))
    }
    fn resolve_coverage_path(&self, node: &ExpandedNode) -> Option<kurbo::BezPath> {
        self.resolve_convex_mesh(node)
            .map(|s| polygon_path(&s.silhouette()))
    }
    fn resolve_coverage_opacity(&self, node: &ExpandedNode) -> f64 {
        if supported(node) {
            1.
        } else {
            0.
        }
    }
    fn ray_cast_test(&self, node: &ExpandedNode, ray: Point2<Window>) -> bool {
        self.resolve_convex_mesh(node)
            .is_some_and(|s| s.depth_at(ray.x, ray.y).is_some())
    }
    fn render(&self, node: &ExpandedNode, ctx: &Rc<RuntimeContext>, rc: &mut dyn RenderContext) {
        if !ctx.is_canvas_node_dirty(&node.id) {
            return;
        }
        let Some(scope) = crate::common::begin_bounded_canvas_node(rc, node, ctx) else {
            return;
        };
        if !rc.is_gpu() || !supported(node) {
            if node.computed_opacity.get() != 0. {
                static WARN: std::sync::Once = std::sync::Once::new();
                WARN.call_once(|| log::warn!("Ellipsoid requires the GPU renderer, opaque color/opacity, and hard clips; alpha masks and effect captures are not supported yet"));
            }
        } else if let Some(solid) = &ctx.canvas_geometry_for_node(node).solid {
            let inverse = ctx
                .canvas_surface_to_world(scope.layer_id)
                .inverse()
                .as_coeffs();
            let surface = Matrix::new([
                inverse[0], inverse[1], 0., 0., inverse[2], inverse[3], 0., 0., 0., 0., 1., 0.,
                inverse[4], inverse[5], 0., 1.,
            ]) * Matrix::new(solid.transform);
            node.with_properties_unwrapped(|p: &mut Ellipsoid| {
                rc.set_node_depth_plane(scope.layer_id, scope.node_id, None);
                rc.draw_convex_mesh(
                    scope.layer_id,
                    OpaqueMeshDraw {
                        mesh: solid.mesh.clone(),
                        world: solid.transform,
                        surface: surface.coeffs(),
                        color: p.color.local().get(),
                        material: p.material.local().get().resolve_in_graph(),
                    },
                );
            });
        }
        if rc.end_node(scope.layer_id, scope.node_id) {
            ctx.clear_canvas_node_dirty(&node.id);
        }
    }
}
