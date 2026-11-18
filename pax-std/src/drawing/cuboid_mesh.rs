use pax_engine::{
    api::{Depth, Fill, Layer, LocalProperty, Property, RenderContext},
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

// Internal Cuboid body. The wrapper keeps layered/transparent paints on planar
// faces and keeps authored CuboidFace subtrees mounted across mode changes.
#[doc(hidden)]
#[pax]
#[custom(Default)]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::cuboid_mesh::CuboidMeshInstance")]
pub struct CuboidMesh {
    /// Absolute local Z extent; defaults to 100 logical pixels.
    pub depth: Property<Depth>,
    pub fill: Property<Vec<Fill>>,
}
impl Default for CuboidMesh {
    fn default() -> Self {
        Self {
            depth: Property::new(Depth::from(100.)),
            fill: Property::new(vec![Fill::default()]),
        }
    }
}
thread_local! {
    static CUBE: Rc<ConvexMesh> = ConvexMesh::cube();
}
type Matrix = Transform3<Generic>;

pub struct CuboidMeshInstance {
    base: BaseInstance,
    gpu: std::cell::Cell<bool>,
    // A template instance can serve several expanded nodes (for example in a
    // repeat). Each mounted solid owns its pose; unmount retires that entry.
    poses: RefCell<HashMap<u32, Rc<PosedConvexMesh>>>,
}

fn supported(node: &ExpandedNode) -> bool {
    let opaque = node.with_properties_unwrapped(|p: &mut CuboidMesh| {
        super::cuboid::mesh_fill(&p.fill.local().get()).is_some()
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
impl InstanceNode for CuboidMeshInstance {
    fn supports_retained_motion(&self) -> bool {
        true
    }
    fn canvas_geometry_is_local(&self) -> bool {
        // The default local paint bound is unbounded; the posed volume is
        // resolved independently for each current world-space record.
        true
    }
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
        f.debug_struct("CuboidMesh").finish()
    }
    fn base(&self) -> &BaseInstance {
        &self.base
    }
    fn handle_mount(self: Rc<Self>, node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        self.gpu.set(context.globals().renderer_is_gpu);
        let (depth, fill) =
            node.with_properties_unwrapped(|p: &mut CuboidMesh| (p.depth.local(), p.fill.local()));
        let appearance =
            crate::common::watch_appearance(node, context, fill, LocalProperty::new(Vec::new()));
        let mut deps = vec![
            depth.untyped(),
            node.opaque_mesh_content_allowed().untyped(),
            appearance.untyped(),
            node.computed_opacity.untyped(),
            node.computed_opacity_scopes.untyped(),
        ];
        deps.extend(node.canvas_placement_dependencies());
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
            node.with_properties_unwrapped(|p: &mut CuboidMesh| p.depth.local().get().to_float());
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
        if let Some(pose) = retained.get(&id).filter(|p| p.transform == world.coeffs()) {
            return Some(pose.clone());
        }
        let pose = CUBE.with(|mesh| mesh.posed(world.coeffs()));
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
        self.resolve_convex_mesh(node)
            .map(pax_runtime_api::HitShape::Mesh)
            .unwrap_or_default()
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
                WARN.call_once(|| log::warn!("CuboidMesh requires the GPU renderer, opaque surface color, and hard clips; translucent materials and alpha-masked solid content are not supported yet"));
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
            node.with_properties_unwrapped(|p: &mut CuboidMesh| {
                rc.set_node_depth_plane(
                    scope.layer_id,
                    scope.node_id,
                    ctx.depth_plane_in_presentation(node, None),
                );
                rc.draw_convex_mesh(
                    scope.layer_id,
                    OpaqueMeshDraw {
                        mesh: solid.mesh.clone(),
                        world: solid.transform,
                        surface,
                        color: super::cuboid::mesh_fill(&p.fill.local().get()).unwrap(),
                        material: p.fill.local().get()[0]
                            .material
                            .local()
                            .get()
                            .resolve_in_graph(),
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
                Some(Rc::new(RefCell::new(CuboidMesh::default().to_pax_any())))
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
    fn cuboid_body_mode_tracks_nested_paints_dimensions_and_stack_replacement() {
        use super::super::cuboid::Cuboid;
        use pax_engine::api::{Color, Paint};
        let context = Rc::new(crate::test_support::runtime_context());
        let _graph = context.property_graph.enter();
        let mut root_args = args();
        root_args.prototypical_properties = PropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(Cuboid::default().to_pax_any())))
        }));
        let root =
            ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
        root.with_properties_unwrapped(|p: &mut Cuboid| {
            p.on_mount(&root.get_node_context(&context))
        });
        root.recurse_update(&context);
        let (mode, fill, depth) = root.with_properties_unwrapped(|p: &mut Cuboid| {
            (p._mesh.local(), p.fill.local(), p.depth.local())
        });
        assert!(mode.get());
        let layer = fill.get()[0].clone();
        layer.opacity.local().set(0.5.into());
        assert!(!mode.get());
        layer.opacity.local().set(1.0.into());
        assert!(mode.get());
        layer.paint.local().set(Paint::Solid(Color::rgba(1.0.into(), 0.0.into(), 0.0.into(), 0.5.into())));
        assert!(!mode.get());
        layer.paint.local().set(Paint::Solid(Color::RED));
        assert!(mode.get());
        depth.set(Depth::from(0.));
        assert!(!mode.get());
        depth.set(Depth::from(40.));
        assert!(mode.get());
        fill.set(vec![Fill::default(), Fill::default()]);
        context.drain_node_effects();
        assert!(!mode.get());
        fill.set(vec![Fill::default()]);
        context.drain_node_effects();
        assert!(mode.get());
        root.recurse_unmount(&context);
    }

    #[test]
    fn repeated_solids_retain_independent_poses_and_release_them_on_unmount() {
        let context = Rc::new(crate::test_support::runtime_context());
        let solid = CuboidMeshInstance::instantiate(args());
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
        nodes[0].with_properties_unwrapped(|p: &mut CuboidMesh| p.depth.set(Depth::from(200.)));
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
