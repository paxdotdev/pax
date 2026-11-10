use pax_engine::{
    api::{Color, Depth, Fill, Layer, LocalProperty, Material, Property, RenderContext},
    pax,
};
use pax_runtime::api as pax_runtime_api;
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use pax_runtime_api::{
    math::{Generic, Transform3},
    mesh3::{ConvexMesh, OpaqueMeshDraw, PosedConvexMesh},
    use_RefCell,
};
use std::collections::HashMap;
use std::rc::Rc;
use_RefCell!();

/// An opaque ellipsoid with width × height × depth extents, centered on local Z=0.
/// Requires `pax-kit/3d` and the GPU renderer. Ordinary transforms, hard clips,
/// lighting, and depth-aware native-control occlusion apply to the complete volume.
/// Root `@camera` pan, zoom, rotation and perspective apply to rendering, picking
/// and occlusion. Near/far clipping cuts the surface without adding caps.
///
/// Node and ancestor opacity fade the assembled opaque appearance through the
/// bounded spatial group compositor; hidden rear surfaces stay hidden.
/// This primitive accepts a solid opaque color. Translucent material color
/// and alpha-masked solid content are unsupported;
/// unsupported instances are omitted with a diagnostic. Zero/negative dimensions
/// or a singular pose produce no coverage. Transparent shells are a separate contract.
/// An ellipsoid can supply a detached alpha-mask source; its projected silhouette,
/// hard clips and assembly opacity contribute to the mask's alpha.
/// With `pax-kit/shaders`, shader containers resolve internal mesh depth before
/// filtering. Their output is a flat image at the container's plane; logical
/// input and native coverage continue to follow the original geometry.
#[pax]
#[custom(Default)]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::ellipsoid::EllipsoidInstance")]
pub struct Ellipsoid {
    /// Absolute local Z extent; defaults to 100 logical pixels.
    pub depth: Property<Depth>,
    /// Opaque surface color. Defaults to white.
    pub color: Property<Color>,
    /// Surface response, unlit by default. Specify a lit material to respond to scene lights.
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
    // A template instance can serve several expanded nodes (for example in a
    // repeat). Each mounted solid owns its pose; unmount retires that entry.
    poses: RefCell<HashMap<u32, Rc<PosedConvexMesh>>>,
}

fn supported(node: &ExpandedNode) -> bool {
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
    fn supports_alpha_source_render(&self) -> bool {
        true
    }
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
            poses: RefCell::new(HashMap::new()),
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
            node.presentation_geometry().untyped(),
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
    fn handle_unmount(&self, node: &Rc<ExpandedNode>, _: &Rc<RuntimeContext>) {
        self.poses.borrow_mut().remove(&node.id.to_u32());
    }
    fn resolve_convex_mesh(&self, node: &ExpandedNode) -> Option<Rc<PosedConvexMesh>> {
        let id = node.id.to_u32();
        if !self.gpu.get() || !supported(node) {
            self.poses.borrow_mut().remove(&id);
            return None;
        }
        let (w, h) = node.transform_and_bounds.get().bounds;
        let d =
            node.with_properties_unwrapped(|p: &mut Ellipsoid| p.depth.local().get().to_float());
        if ![w, h, d].iter().all(|v| v.is_finite() && *v > 0.) {
            self.poses.borrow_mut().remove(&id);
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
        let mut retained = self.poses.borrow_mut();
        if let Some(pose) = retained
            .get(&id)
            .filter(|pose| pose.transform == world.coeffs())
        {
            return Some(pose.clone());
        }
        let pose = SPHERE.with(|mesh| mesh.posed(world.coeffs()));
        if let Some(pose) = &pose {
            retained.insert(id, pose.clone());
        } else {
            retained.remove(&id);
        }
        pose
    }
    fn resolve_coverage_path(&self, node: &ExpandedNode) -> Option<kurbo::BezPath> {
        self.resolve_convex_mesh(node)
            .and_then(|s| node.project_convex_mesh(&s))
            .map(|s| polygon_path(&s.silhouette()))
    }
    fn resolve_coverage_opacity(&self, node: &ExpandedNode) -> f64 {
        if supported(node) {
            node.computed_opacity.get().clamp(0., 1.)
        } else {
            0.
        }
    }
    fn prepare_hit_shape(&self, node: &ExpandedNode) -> pax_runtime_api::HitShape {
        self.resolve_convex_mesh(node).map(pax_runtime_api::HitShape::Mesh).unwrap_or_default()
    }
    fn render(&self, node: &ExpandedNode, ctx: &Rc<RuntimeContext>, rc: &mut dyn RenderContext) {
        if rc.alpha_source_layer().is_none() && !ctx.is_canvas_node_dirty(&node.id) {
            return;
        }
        let Some(scope) = crate::common::begin_bounded_canvas_node(rc, node, ctx) else {
            return;
        };
        if !rc.is_gpu() || !supported(node) {
            if node.computed_opacity.get() != 0. {
                static WARN: std::sync::Once = std::sync::Once::new();
                WARN.call_once(|| log::warn!("Ellipsoid requires the GPU renderer, opaque surface color, and hard clips; translucent materials and alpha-masked solid content are not supported yet"));
            }
        } else if let Some(solid) = &ctx.canvas_geometry_for_node(node).solid {
            let surface = if node.is_render_source() && node.has_perspective_camera() {
                ctx.canvas_mesh_projection_on_layer(node, scope.layer_id, solid.transform)
            } else {
                ctx.canvas_geometry_for_node(node).solid_projection
            };
            let Some(surface) = surface else {
                rc.end_node(scope.layer_id, scope.node_id);
                return;
            };
            node.with_properties_unwrapped(|p: &mut Ellipsoid| {
                rc.set_node_depth_plane(scope.layer_id, scope.node_id, ctx.depth_plane_in_presentation(node, None));
                rc.draw_convex_mesh(
                    scope.layer_id,
                    OpaqueMeshDraw {
                        mesh: solid.mesh.clone(),
                        world: solid.transform,
                        surface,
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

#[cfg(test)]
mod tests {
    use super::*;
    use pax_runtime::api::{pax_value::ToFromPaxAny, CommonProperties, Size};
    use pax_runtime::{
        CommonPropertiesInit, ComponentInstance, PropertiesInit, PropertiesScopeInit,
    };

    #[test]
    fn ellipsoid_requires_explicit_lighting_opt_in() {
        let surface = Ellipsoid::default();
        assert_eq!(surface.material.get(), Material::Unlit);
        surface.material.set(Material::matte());
        assert!(matches!(surface.material.get(), Material::Lit(_)));
    }

    fn args() -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(|_, _| {
                Some(Rc::new(RefCell::new(CommonProperties {
                    width: LocalProperty::new(Some(Size::Pixels(100.into()))),
                    height: LocalProperty::new(Some(Size::Pixels(100.into()))),
                    ..Default::default()
                })))
            })),
            prototypical_properties: PropertiesInit::Factory(Box::new(|_, _| {
                Some(Rc::new(RefCell::new(Ellipsoid::default().to_pax_any())))
            })),
            handler_registry: None,
            children: None,
            component_template: None,
            component_settings: None,
            component_camera: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: PropertiesScopeInit::None,
        }
    }

    #[test]
    fn repeated_solids_retain_independent_poses_and_release_them_on_unmount() {
        let context = Rc::new(crate::test_support::runtime_context());
        let solid = EllipsoidInstance::instantiate(args());
        let mut root_args = args();
        root_args.component_template = Some(RefCell::new(vec![solid.clone(), solid.clone()]));
        let root =
            ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
        root.recurse_update(&context);
        let nodes = root.children.get();
        let first = solid.resolve_convex_mesh(&nodes[0]).unwrap();
        let second = solid.resolve_convex_mesh(&nodes[1]).unwrap();
        nodes[0]
            .common_properties
            .borrow()
            .borrow()
            .opacity
            .set(Some(0.4.into()));
        root.recurse_update(&context);
        assert!((nodes[0].computed_opacity.get() - 0.4).abs() < 1e-9);
        assert!(Rc::ptr_eq(
            &first,
            &solid.resolve_convex_mesh(&nodes[0]).unwrap()
        ));
        nodes[0].with_properties_unwrapped(|p: &mut Ellipsoid| p.depth.set(Depth::from(200.)));
        root.recurse_update(&context);
        let changed = solid.resolve_convex_mesh(&nodes[0]).unwrap();
        assert!(!Rc::ptr_eq(&first, &changed));
        assert!(Rc::ptr_eq(
            &second,
            &solid.resolve_convex_mesh(&nodes[1]).unwrap()
        ));
        let weak = Rc::downgrade(&changed);
        drop(changed);
        nodes[0].clone().recurse_unmount(&context);
        assert!(!solid.poses.borrow().contains_key(&nodes[0].id.to_u32()));
        assert!(weak.upgrade().is_none());
        assert!(Rc::ptr_eq(
            &second,
            &solid.resolve_convex_mesh(&nodes[1]).unwrap()
        ));
        root.recurse_unmount(&context);
        assert!(solid.poses.borrow().is_empty());
    }
}
