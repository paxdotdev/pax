use super::*;
use crate::api::{CommonProperties, Layer, Rotation, Size, Transform2D};
use crate::node_interface::NodeInterface;
use crate::scene_geometry::CanvasGeometry;
use crate::*;
use pax_runtime_api::math::{Point3, Vector2, Vector3};
use pax_runtime_api::pax_value::PaxAny;
use pax_runtime_api::{LocalProperty, Platform, OS};
use std::{cell::RefCell, f64::consts::FRAC_PI_2, rc::Rc};

fn close(a: f64, b: f64) {
    assert!(
        (a - b).abs() < 1e-9 * (1. + a.abs().max(b.abs())),
        "{a} != {b}"
    );
}

fn same_transform(a: Transform2<NodeLocal, Window>, b: Transform2<NodeLocal, Window>) {
    for (a, b) in a.coeffs().into_iter().zip(b.coeffs()) {
        close(a, b);
    }
}

#[test]
fn child_composes_before_projection_even_beneath_an_edge_on_parent() {
    // Exact quarter turns make the parent's projection singular, without relying
    // on cos(pi/2) rounding. A counter-rotated child must recover its full width.
    let quarter_turn: Transform3<NodeLocal, Window> = Transform3::new([
        0., 0., -1., 0., 0., 1., 0., 0., 1., 0., 0., 0., 40., 50., 20., 1.,
    ]);
    let parent = SpatialGeometry::new(quarter_turn, (200., 100.));
    assert_eq!(parent.orthographic_projection().unwrap().transform.m[0], 0.);
    let child = parent.append_local(
        Transform3::<NodeLocal>::translate(Vector3::new(10., 15., 5.))
            * Transform3::rotate_y(-FRAC_PI_2),
        (80., 30.),
    );
    let projected = child.orthographic_projection().unwrap();
    same_transform(
        projected.transform,
        Transform2::translate(Vector2::new(45., 65.)),
    );
    assert_eq!(projected.bounds, (80., 30.));
    let origin = child
        .world_transform()
        .project_point(Point3::new(0., 0., 0.))
        .unwrap();
    close(origin.z, 10.);
    assert_eq!(parent.is_coplanar_with(child, 1e-9, 1e-9), Some(false));
}

#[test]
fn geometric_plane_and_facing_are_independent() {
    let group = SpatialGeometry::new(Transform3::rotate_y(0.7), (200., 100.));
    let a = group.append_local(
        Transform3::translate(Vector3::new(10., 20., 0.)),
        (40., 30.),
    );
    let b = group.append_local(
        Transform3::translate(Vector3::new(80., 20., 0.)),
        (40., 30.),
    );
    assert_eq!(a.is_coplanar_with(b, 1e-9, 1e-9), Some(true));
    let tilted = b.append_local(Transform3::rotate_x(0.2), b.bounds);
    assert_eq!(a.is_coplanar_with(tilted, 1e-9, 1e-9), Some(false));
    let offset = b.append_local(Transform3::translate(Vector3::new(0., 0., 0.01)), b.bounds);
    assert_eq!(a.is_coplanar_with(offset, 1e-9, 1e-9), Some(false));
    let back = b.append_local(Transform3::scale(Vector3::new(-1., 1., 1.)), b.bounds);
    assert_eq!(a.is_coplanar_with(back, 1e-9, 1e-9), Some(true));
    close(
        a.plane()
            .unwrap()
            .normal()
            .dot(back.plane().unwrap().normal()),
        -1.,
    );
    let restored = tilted.append_local(Transform3::rotate_x(-0.2), b.bounds);
    assert_eq!(a.is_coplanar_with(restored, 1e-9, 1e-9), Some(true));
}

#[test]
fn degenerate_planes_and_projective_transforms_are_not_silently_flattened() {
    let collapsed_xy =
        SpatialGeometry::new(Transform3::scale(Vector3::new(0., 1., 1.)), (20., 30.));
    assert!(collapsed_xy.plane().is_none());
    assert_eq!(
        collapsed_xy.is_coplanar_with(SpatialGeometry::default(), 0., 0.),
        None
    );
    let collapsed_z = SpatialGeometry::new(Transform3::scale(Vector3::new(1., 1., 0.)), (20., 30.));
    assert!(collapsed_z.plane().is_some());
    let mut perspective = Transform3::<NodeLocal, Window>::identity().coeffs();
    perspective[3] = 0.1;
    let perspective = SpatialGeometry::new(Transform3::new(perspective), (20., 30.));
    assert!(perspective.orthographic_projection().is_none());
    assert!(perspective.plane().is_none());
    let mut invalid = Transform3::<NodeLocal, Window>::identity().coeffs();
    invalid[14] = f64::NAN;
    assert!(SpatialGeometry::new(Transform3::new(invalid), (20., 30.))
        .plane()
        .is_none());
}

#[test]
fn ordinary_trees_keep_affine_arithmetic_and_exact_spatial_opt_in() {
    let parent = SpatialGeometry::from(TransformAndBounds {
        transform: Transform2::<NodeLocal, Window>::translate(Vector2::new(23., 41.))
            * Transform2::rotate(0.3),
        bounds: (120., 90.),
    });
    let local = TransformAndBounds {
        transform: Transform2::<NodeLocal>::rotate(-0.7)
            * Transform2::scale_sep(Vector2::new(1.2, 0.3)),
        bounds: (40., 20.),
    };
    let composed = parent.append_local_2d(local);
    assert!(matches!(composed.transform, RetainedTransform::Planar(_)));
    assert_eq!(
        composed.orthographic_projection().unwrap().transform,
        parent.orthographic_projection().unwrap().transform * local.transform
    );
    let tiny_depth = parent.append_local(
        Transform3::translate(Vector3::new(0., 0., 1e-15)),
        local.bounds,
    );
    assert!(matches!(
        tiny_depth.transform,
        RetainedTransform::Spatial(_)
    ));
    assert_eq!(tiny_depth.world_transform().coeffs()[14], 1e-15);
}

fn args(children: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Default,
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, node| {
            node.is_none()
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
        })),
        handler_registry: None,
        children: Some(RefCell::new(children)),
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

struct Surface(BaseInstance, bool);
impl InstanceNode for Surface {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self(
            BaseInstance::new(
                args,
                InstanceFlags {
                    layer: Layer::Canvas,
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    is_component: false,
                    is_slot: false,
                },
            ),
            false,
        ))
    }
    fn scrolls_content(&self, _: &ExpandedNode) -> bool {
        self.1
    }
    fn resolve_scroll_offset(&self, _: &ExpandedNode) -> Option<(f64, f64)> {
        self.1.then_some((20.0, 0.0))
    }
    fn base(&self) -> &BaseInstance {
        &self.0
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Surface").finish()
    }
    fn prepare_canvas_geometry(&self, node: &ExpandedNode) -> CanvasGeometry {
        CanvasGeometry::for_layout(node.transform_and_bounds.get().bounds)
    }
}

fn fixture() -> (
    PaxEngine,
    Rc<ExpandedNode>,
    Rc<ExpandedNode>,
    Rc<ExpandedNode>,
) {
    fixture_with_camera(None)
}

fn fixture_with_camera(
    camera: Option<pax_manifest::LiteralBlockDefinition>,
) -> (
    PaxEngine,
    Rc<ExpandedNode>,
    Rc<ExpandedNode>,
    Rc<ExpandedNode>,
) {
    let leaf = Surface::instantiate(args(vec![]));
    let a = Surface::instantiate(args(vec![leaf]));
    let b = Surface::instantiate(args(vec![]));
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![a, b]));
    root_args.component_camera = camera;
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.runtime_context.resize_canvas_layers_to(1);
    engine.tick();
    let a = root.children.get()[0].clone();
    let b = root.children.get()[1].clone();
    let child = a.children.get()[0].clone();
    (engine, a, b, child)
}

fn common(node: &Rc<ExpandedNode>) -> CommonProperties {
    node.get_common_properties().borrow().clone()
}

fn legacy_geometry(
    node: &Rc<ExpandedNode>,
    viewport: TransformAndBounds<NodeLocal, Window>,
) -> TransformAndBounds<NodeLocal, Window> {
    let parent = node.render_parent_node();
    let geometry = parent
        .as_ref()
        .map(|parent| legacy_geometry(parent, viewport))
        .unwrap_or(viewport);
    let (px, py) = parent
        .as_ref()
        .map(|parent| {
            let cp = common(parent);
            (cp.padding_x.get(), cp.padding_y.get())
        })
        .unwrap_or_default();
    let container = apply_container_frame(
        apply_padding_frame(geometry, px, py),
        node.container_frame.get(),
    );
    let geometry = calculate_transform_and_bounds(&node.layout_properties().get(), container);
    common(node)
        .transform
        .get()
        .map(|extra| extra.apply(geometry))
        .unwrap_or(geometry)
}

#[test]
fn mounted_geometry_preserves_legacy_layout_and_prepared_records() {
    let (mut engine, parent, _, child) = fixture();
    let parent_cp = common(&parent);
    parent_cp.x.set(Some(Size::Percent(50.into())));
    parent_cp.y.set(Some(Size::Percent(50.into())));
    parent_cp.width.set(Some(Size::Pixels(320.into())));
    parent_cp.height.set(Some(Size::Pixels(220.into())));
    parent_cp.rotate.set(Some(Rotation::Degrees(31.into())));
    parent_cp.scale_x.set(Some(Size::Percent(130.into())));
    parent_cp.padding_x.set(Some(Size::Percent(10.into())));
    parent_cp.padding_y.set(Some(Size::Pixels(11.into())));
    let cp = common(&child);
    cp.x.set(Some(Size::Percent(75.into())));
    cp.y.set(Some(Size::Pixels(27.into())));
    cp.width.set(Some(Size::Percent(40.into())));
    cp.height.set(Some(Size::Pixels(60.into())));
    cp.skew_x.set(Some(Rotation::Degrees(8.into())));
    cp.anchor_y.set(Some(Size::Percent(50.into())));
    cp.transform.set(Some(
        Transform2D::rotate(Rotation::Degrees(17.into()))
            * Transform2D::translate(Size::Pixels(5.into()), Size::Percent(20.into())),
    ));
    child.container_frame.set(Some(ContainerFrame {
        transform: Transform2::<NodeLocal>::translate(Vector2::new(13., 19.))
            * Transform2::rotate(-0.2),
        bounds: (190., 140.),
    }));
    engine.tick();
    let expected = legacy_geometry(&child, engine.runtime_context.globals().viewport.get());
    let actual = child.transform_and_bounds.get();
    same_transform(actual.transform, expected.transform);
    assert_eq!(actual.bounds, expected.bounds);
    let prepared = engine.runtime_context.canvas_geometry_for_node(&child);
    same_transform(
        prepared
            .spatial_geometry
            .orthographic_projection()
            .unwrap()
            .transform,
        expected.transform,
    );
    let preparations = engine.runtime_context.scene_geometry_stats().preparations;
    assert!(Rc::ptr_eq(
        &prepared,
        &engine.runtime_context.canvas_geometry_for_node(&child)
    ));
    assert_eq!(
        engine.runtime_context.scene_geometry_stats().preparations,
        preparations
    );
}

#[test]
fn parent_pose_does_not_recompute_local_layout_and_collapse_keeps_local_placement() {
    let (mut engine, parent, _, child) = fixture();
    common(&child).x.set(Some(Size::Pixels(37.into())));
    engine.tick();
    let interface = NodeInterface::from(child.clone());
    let local = interface.local_transform_and_bounds();
    let runs = Rc::new(std::cell::Cell::new(0));
    let observed = {
        let runs = runs.clone();
        let local = local.clone();
        let deps = [local.untyped()];
        LocalProperty::computed(
            move || {
                runs.set(runs.get() + 1);
                local.get().bounds
            },
            &deps,
        )
    };
    observed.get();
    let before = local.get();
    common(&parent).x.set(Some(Size::Pixels(99.into())));
    common(&parent).scale_x.set(Some(Size::Percent(0.into())));
    engine.tick();
    observed.get();
    assert_eq!(
        runs.get(),
        1,
        "ancestor pose must not invalidate local placement"
    );
    assert_eq!(local.get().transform, before.transform);
    assert!(interface.spatial_geometry().get().plane().is_none());
    common(&child).x.set(Some(Size::Pixels(53.into())));
    assert_eq!(local.get().transform.m[4], 53.);
    common(&parent).scale_x.set(None);
    engine.tick();
    close(child.transform_and_bounds.get().transform.m[4], 152.);
}

#[test]
fn local_and_world_dimensions_update_synchronously_before_the_effect_drain() {
    let (mut engine, parent, _, child) = fixture();
    let parent_cp = common(&parent);
    common(&child).width.set(Some(Size::Percent(50.into())));
    parent_cp.width.set(Some(Size::Pixels(200.into())));
    let local = NodeInterface::from(child.clone()).local_transform_and_bounds();
    assert_eq!(local.get().bounds.0, 100.);
    assert_eq!(child.transform_and_bounds.get().bounds.0, 100.);
    parent_cp.width.set(Some(Size::Pixels(300.into())));
    assert_eq!(local.get().bounds.0, 150.);
    assert_eq!(child.transform_and_bounds.get().bounds.0, 150.);
    parent.set_measured_size(180., 120.);
    parent_cp.width.set(None);
    assert_eq!(local.get().bounds.0, 90.);
    assert_eq!(child.spatial_geometry.get().bounds.0, 90.);
    parent.measured_size.set(None);
    engine.set_viewport_size((800., 600.));
    assert_eq!(local.get().bounds.0, 400.);
    assert_eq!(child.spatial_geometry.get().bounds.0, 400.);
}

#[test]
fn depth_only_world_change_invalidates_prepared_geometry_without_changing_footprint() {
    let (mut engine, parent, _, child) = fixture();
    let before = engine.runtime_context.canvas_geometry_for_node(&child);
    // Exercise depth-only invalidation without depending on a projected XY change.
    parent.spatial_geometry.set(SpatialGeometry::new(
        Transform3::translate(Vector3::new(0., 0., 12.)),
        parent.spatial_geometry.get().bounds,
    ));
    engine.tick();
    let after = engine.runtime_context.canvas_geometry_for_node(&child);
    assert!(!Rc::ptr_eq(&before, &after));
    assert_eq!(before.world_transform, after.world_transform);
    assert_eq!(before.coverage_bounds, after.coverage_bounds);
    close(after.spatial_geometry.plane().unwrap().origin().z, 12.);
    assert_eq!(
        before
            .spatial_geometry
            .is_coplanar_with(after.spatial_geometry, 1e-9, 1e-9),
        Some(false)
    );
}

#[test]
fn reparenting_rebinds_retained_geometry_without_replacing_node_or_property_handles() {
    let (mut engine, a, b, child) = fixture();
    common(&a).x.set(Some(Size::Pixels(20.into())));
    common(&b).x.set(Some(Size::Pixels(150.into())));
    common(&b).padding_x.set(Some(Size::Pixels(10.into())));
    common(&child).x.set(Some(Size::Pixels(7.into())));
    engine.tick();
    let handle = NodeInterface::from(child.clone()).spatial_geometry();
    let local = NodeInterface::from(child.clone()).local_transform_and_bounds();
    let id = child.id;
    close(
        handle.get().orthographic_projection().unwrap().transform.m[4],
        27.,
    );
    b.attach_children(
        vec![child.clone()],
        &engine.runtime_context,
        &b.parent_frame,
    );
    close(
        handle.get().orthographic_projection().unwrap().transform.m[4],
        167.,
    );
    close(local.get().transform.m[4], 17.);
    assert_eq!(child.id, id);
    assert_eq!(child.attached.get(), 1);
    common(&a).x.set(Some(Size::Pixels(500.into())));
    close(
        handle.get().orthographic_projection().unwrap().transform.m[4],
        167.,
    );
    common(&b).x.set(Some(Size::Pixels(200.into())));
    close(
        handle.get().orthographic_projection().unwrap().transform.m[4],
        217.,
    );
}

#[test]
fn authored_spatial_properties_preserve_pivots_and_local_layout() {
    let (mut engine, parent, _, child) = fixture();
    let cp = common(&parent);
    cp.x.set(Some(Size::Pixels(220.into())));
    cp.y.set(Some(Size::Pixels(170.into())));
    cp.width.set(Some(Size::Pixels(200.into())));
    cp.height.set(Some(Size::Pixels(100.into())));
    cp.anchor_x.set(Some(Size::Percent(50.into())));
    cp.anchor_y.set(Some(Size::Percent(50.into())));
    cp.rotate_y.set(Some(Rotation::Degrees(60.into())));
    cp.z.set(Some(pax_runtime_api::Depth::from(25.0)));
    engine.tick();
    let geometry = parent.spatial_geometry.get();
    let pivot = geometry
        .world_transform()
        .project_point(Point3::new(100.0, 50.0, 0.0))
        .unwrap();
    close(pivot.x, 220.0);
    close(pivot.y, 170.0);
    close(pivot.z, 25.0);
    assert_eq!(parent.layout_bounds.get(), (200.0, 100.0));
    assert_eq!(child.layout_bounds.get(), (200.0, 100.0));
    let projected = geometry.orthographic_projection().unwrap();
    let depth = geometry.depth_plane(projected.transform).unwrap();
    close(depth.depth_at(pivot.x, pivot.y), 25.0);
    let corner = geometry
        .world_transform()
        .project_point(Point3::new(0.0, 0.0, 0.0))
        .unwrap();
    close(depth.depth_at(corner.x, corner.y), corner.z);
    cp.rotate_y.set(Some(Rotation::Degrees(0.into())));
    cp.z.set(Some(pax_runtime_api::Depth::from(0.0)));
    engine.tick();
    same_transform(
        parent.transform_and_bounds.get().transform,
        legacy_geometry(&parent, engine.runtime_context.globals().viewport.get()).transform,
    );
    assert!(parent
        .spatial_geometry
        .get()
        .world_transform()
        .try_as_2d()
        .is_some());
}

#[test]
fn front_region_splits_native_coverage_and_reverses_backface_order() {
    use kurbo::Shape;
    use pax_runtime_api::DepthPlane;
    let flat = DepthPlane::default();
    let tilted = DepthPlane {
        presentation_layer: 0,
        coefficients: [1.0, 0.0, -50.0],
        backface: false,
    };
    let bounds = kurbo::Rect::new(0.0, 0.0, 100.0, 100.0);
    let overlay = DepthPlane {
        presentation_layer: 1,
        ..flat
    };
    let high_world = DepthPlane {
        coefficients: [100., 100., 1e9],
        ..flat
    };
    close(
        front_region(overlay, high_world, -100, 100, bounds)
            .unwrap()
            .area(),
        10000.,
    );
    assert!(front_region(high_world, overlay, 100, -100, bounds).is_none());
    let region = front_region(tilted, flat, 0, 10, bounds).unwrap();
    close(region.area(), 5000.0);
    assert!(!region.contains((25.0, 50.0).into()));
    assert!(region.contains((75.0, 50.0).into()));
    assert!(front_region(flat, flat, 0, 10, bounds).is_none());
    let back = DepthPlane {
        backface: true,
        ..flat
    };
    assert!(front_region(back, flat, 20, 10, bounds).is_none());
    assert!(front_region(flat, back, -20, -10, bounds).is_some());
    close(
        front_region(back, back, 0, 10, bounds).unwrap().area(),
        10000.0,
    );
}

#[test]
fn input_uses_geometric_depth_at_the_pointer() {
    use pax_runtime_api::math::Point2;
    let (mut engine, a, b, child) = fixture();
    common(&child)._raycastable.set(Some(false));
    for node in [&a, &b] {
        let cp = common(node);
        cp.x.set(Some(Size::Pixels(150.into())));
        cp.y.set(Some(Size::Pixels(100.into())));
        cp.width.set(Some(Size::Pixels(200.into())));
        cp.height.set(Some(Size::Pixels(100.into())));
        cp.anchor_x.set(Some(Size::Percent(50.into())));
        cp.anchor_y.set(Some(Size::Percent(50.into())));
    }
    common(&a).rotate_y.set(Some(Rotation::Degrees(45.into())));
    engine.tick();
    for (x, expected) in [(110.0, a.id), (190.0, b.id)] {
        let hits = engine.runtime_context.get_elements_beneath_ray(
            None,
            Point2::new(x, 100.0),
            true,
            vec![],
            false,
        );
        assert_eq!(hits[0].id, expected);
    }
    common(&b).z.set(Some(pax_runtime_api::Depth::from(100.0)));
    engine.tick();
    let hits = engine.runtime_context.get_elements_beneath_ray(
        None,
        Point2::new(110.0, 100.0),
        true,
        vec![],
        false,
    );
    assert_eq!(hits[0].id, b.id);
    // Coincident opposing faces use the renderer's tie, even though A is
    // earlier in authored order.
    common(&b).z.set(Some(pax_runtime_api::Depth::from(0.0)));
    common(&a).rotate_y.set(Some(Rotation::Degrees(180.into())));
    engine.tick();
    let hit = || {
        engine.runtime_context.get_elements_beneath_ray(
            None,
            Point2::new(150.0, 100.0),
            true,
            vec![],
            false,
        )[0]
        .id
    };
    assert_eq!(hit(), b.id);
    engine
        .runtime_context
        .edit_globals(|globals| globals.renderer_is_gpu = false);
    assert_eq!(hit(), a.id);
}

#[test]
fn invalid_and_edge_on_poses_keep_layout_and_recover_without_remounting() {
    use pax_runtime_api::math::Point2;
    let (mut engine, parent, _, child) = fixture();
    let cp = common(&parent);
    cp.rotate_y.set(Some(Rotation::Degrees(90.into())));
    engine.tick();
    assert!(!parent.ray_cast_test(Point2::new(0.0, 0.0)));
    let original_bounds = child.layout_bounds.get();
    cp.z.set(Some(pax_runtime_api::Depth::from(f64::NAN)));
    engine.tick();
    assert_eq!(
        parent.transform_and_bounds.get().transform.coeffs(),
        [0.0; 6]
    );
    assert_eq!(child.layout_bounds.get(), original_bounds);
    assert!(!parent.ray_cast_test(Point2::new(0.0, 0.0)));
    cp.z.set(Some(pax_runtime_api::Depth::from(0.0)));
    cp.rotate_y.set(Some(Rotation::Degrees(0.into())));
    engine.tick();
    assert_eq!(parent.children.get()[0].id, child.id);
    assert!(parent.ray_cast_test(Point2::new(20.0, 20.0)));
}

#[test]
fn scrolling_on_a_tilted_plane_preserves_its_world_depth_plane() {
    use kurbo::Affine;
    let leaf = Surface::instantiate(args(vec![]));
    let scroller: Rc<dyn InstanceNode> = Rc::new(Surface(
        BaseInstance::new(
            args(vec![leaf]),
            InstanceFlags {
                layer: Layer::Canvas,
                invisible_to_slot: false,
                invisible_to_raycasting: false,
                is_component: false,
                is_slot: false,
            },
        ),
        true,
    ));
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![scroller]));
    let mut engine = crate::test_support::empty_engine(
        (640.0, 480.0),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.runtime_context.resize_canvas_layers_to(2);
    engine.tick();
    let scroller = root.children.get()[0].clone();
    let child = scroller.children.get()[0].clone();
    common(&scroller)
        .rotate_y
        .set(Some(Rotation::Degrees(45.into())));
    engine.tick();
    let expected = scroller
        .spatial_geometry
        .get()
        .depth_plane(scroller.transform_and_bounds.get().transform)
        .unwrap();
    let ctx = &engine.runtime_context;
    let presented = ctx.presentation_scroll_transform_for_node(&child)
        * Affine::from(child.transform_and_bounds.get().transform);
    let actual = ctx.presented_depth_plane(&child, presented).unwrap();
    for (a, b) in actual.coefficients.into_iter().zip(expected.coefficients) {
        close(a, b);
    }
    common(&scroller)
        .rotate_y
        .set(Some(Rotation::Degrees(90.into())));
    engine.tick();
    assert!(engine
        .runtime_context
        .scroller_content_presentation_transform(&scroller)
        .as_coeffs()
        .iter()
        .all(|v| v.is_finite()));
}

#[test]
fn lighting_frame_retains_depth_and_perpendicular_normals_under_shear() {
    let t = Transform3::<NodeLocal, Window>::translate(Vector3::new(12., 17., 80.))
        * Transform3::<NodeLocal>::rotate_y(0.65)
        * Transform3::<NodeLocal>::new([
            2., 0., 0., 0., 0.7, 3., 0., 0., 0., 0., 0., 0., 0., 0., 0., 1.,
        ]);
    let g = SpatialGeometry::new(t, (20., 30.));
    let s = g.lighting_surface().unwrap();
    assert_eq!(s.origin, [12., 17., 80.]);
    for tangent in [s.tangent_x, s.tangent_y] {
        let dot: f32 = tangent.into_iter().zip(s.normal).map(|(a, b)| a * b).sum();
        assert!(dot.abs() < 1e-6);
    }
    assert!((s.normal.iter().map(|n| n * n).sum::<f32>() - 1.).abs() < 1e-6);
    let p = t.project_point(Point3::new(3., 5., 0.)).unwrap();
    for (i, expected) in [p.x, p.y, p.z].into_iter().enumerate() {
        let actual = s.origin[i] + s.tangent_x[i] * 3. + s.tangent_y[i] * 5.;
        assert!((actual as f64 - expected).abs() < 1e-5);
    }
    let flat = SpatialGeometry::new(Transform3::scale(Vector3::new(-2., 3., 1.)), (20., 30.));
    assert_eq!(flat.lighting_surface().unwrap().normal, [0., 0., 1.]);
}

fn camera(values: &[(&str, &str)]) -> pax_manifest::LiteralBlockDefinition {
    use pax_manifest::{
        ExpressionInfo, LiteralBlockDefinition, SettingElement, Token, ValueDefinition,
    };
    LiteralBlockDefinition::new(
        values
            .iter()
            .map(|(key, source)| {
                SettingElement::Setting(
                    Token::new_without_location((*key).into()),
                    ValueDefinition::Expression(ExpressionInfo::new(
                        pax_language::parse_pax_expression(source).unwrap(),
                    )),
                )
            })
            .collect(),
    )
}

#[test]
fn camera_defaults_are_exact_and_view_changes_leave_layout_and_mounts_intact() {
    let (mut engine, parent, _, child) = fixture_with_camera(Some(camera(&[])));
    let root = engine.root_expanded_node.as_ref().unwrap().clone();
    let props = common(&child);
    props.x.set(Some(Size::Pixels(150.into())));
    props.y.set(Some(Size::Pixels(90.into())));
    props.width.set(Some(Size::Pixels(100.into())));
    props.height.set(Some(Size::Pixels(80.into())));
    engine.tick();
    let before = child.transform_and_bounds.get();
    let local = child.local_transform_and_bounds.get();
    let hull = parent.subtree_layout_hull.get();
    assert_eq!(
        child.presentation_transform_and_bounds.get().transform,
        before.transform
    );
    let view =
        crate::camera::resolve_for_test(root.spatial_geometry.get(), [340., 250., 0.], [0.; 3], 2.);
    root.own_camera_view.set(Some(view));
    engine.tick();
    assert_eq!(child.transform_and_bounds.get().transform, before.transform);
    assert_eq!(
        child.local_transform_and_bounds.get().transform,
        local.transform
    );
    assert_eq!(parent.subtree_layout_hull.get(), hull);
    assert!(Rc::ptr_eq(&child, &parent.children.get()[0]));
    let projected = child.presentation_transform_and_bounds.get();
    close(projected.transform.m[4], -60.);
    close(projected.transform.m[5], -80.);
    let window_point = projected.transform * pax_runtime_api::math::Point2::new(25., 30.);
    let local_point = child
        .get_node_context(&engine.runtime_context)
        .local_point(window_point);
    close(local_point.x, 0.25);
    close(local_point.y, 0.375);
    let prepared = engine.runtime_context.canvas_geometry_for_node(&child);
    assert_eq!(
        prepared.world_transform.as_coeffs(),
        projected.transform.coeffs()
    );
}

#[test]
fn root_camera_binds_reactive_pose_and_recovers_from_invalid_zoom() {
    let (mut engine, _, _, child) = fixture_with_camera(Some(camera(&[
        ("position", "[400px, 200px]"),
        ("x", "320px"),
    ])));
    let root = engine.root_expanded_node.as_ref().unwrap().clone();
    close(
        child.presentation_transform_and_bounds.get().transform.m[4],
        0.,
    );
    close(
        child.presentation_transform_and_bounds.get().transform.m[5],
        40.,
    );
    // Rebinding a changed declaration uses the mounted component scope, without replacing nodes.
    let zoom = LocalProperty::new(2.0_f64);
    let mut updated = args(vec![]);
    updated.component_camera = Some(camera(&[("zoom", "camera_zoom")]));
    let zoom_binding = zoom.clone();
    updated.properties_scope = PropertiesScopeInit::Factory(Box::new(move |_| {
        std::collections::HashMap::from([(
            "camera_zoom".into(),
            pax_runtime_api::Variable::new_from_typed_property(zoom_binding.clone()),
        )])
    }));
    root.recreate_with_new_data(
        ComponentInstance::instantiate(updated),
        &engine.runtime_context,
    );
    engine.tick();
    close(
        child.presentation_transform_and_bounds.get().transform.m[0],
        2.,
    );
    zoom.set(0.);
    engine.tick();
    assert_eq!(
        child.presentation_transform_and_bounds.get().transform,
        child.transform_and_bounds.get().transform
    );
    zoom.set(1.5);
    engine.tick();
    close(
        child.presentation_transform_and_bounds.get().transform.m[0],
        1.5,
    );
    assert!(Rc::ptr_eq(
        &child,
        &root.children.get()[0].children.get()[0]
    ));
}

#[test]
fn orthographic_camera_rotates_retained_depth_before_projection() {
    let (mut engine, parent, _, child) =
        fixture_with_camera(Some(camera(&[("rotate_y", "90deg")])));
    common(&parent)
        .z
        .set(Some(pax_runtime_api::Depth(100.into())));
    common(&child).x.set(Some(Size::Pixels(30.into())));
    engine.tick();
    close(child.transform_and_bounds.get().transform.m[4], 30.);
    close(
        child.presentation_transform_and_bounds.get().transform.m[4],
        220.,
    );
    assert!(!child.ray_cast_test(pax_runtime_api::math::Point2::new(220., 20.)));
}

#[test]
fn component_camera_is_dormant_when_embedded_and_active_when_mounted_as_root() {
    let mut nested_args = args(vec![]);
    nested_args.component_template = Some(RefCell::new(vec![Surface::instantiate(args(vec![]))]));
    nested_args.component_camera = Some(camera(&[("zoom", "2")]));
    let nested_instance = ComponentInstance::instantiate(nested_args);
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![nested_instance.clone()]));
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    let nested = root.children.get()[0].clone();
    let child = nested.children.get()[0].clone();
    assert_eq!(
        child.presentation_transform_and_bounds.get().transform,
        child.transform_and_bounds.get().transform
    );
    #[cfg(feature = "designtime")]
    {
        engine
            .runtime_context
            .set_userland_root_expanded_node(Some(nested.clone()));
        engine.tick();
        close(
            child.presentation_transform_and_bounds.get().transform.m[0],
            2.,
        );
        close(
            root.presentation_transform_and_bounds.get().transform.m[0],
            1.,
        );
    }
    let root = engine.mount_root_component(nested_instance);
    engine.tick();
    close(
        root.children.get()[0]
            .presentation_transform_and_bounds
            .get()
            .transform
            .m[0],
        2.,
    );
}

#[test]
fn camera_only_reveal_updates_tile_visibility_without_scroll_velocity() {
    use crate::engine::{
        surface_preparation::{prepare_hosts, PreparationHost, PreparationPolicy},
        surface_scheduling::{CoverageRect, HostObservation, ScrollActivityTracker},
    };
    use std::collections::BTreeMap;
    let (mut engine, host, _, _) = fixture_with_camera(Some(camera(&[])));
    let root = engine.root_expanded_node.as_ref().unwrap().clone();
    common(&host).width.set(Some(Size::Pixels(1000.into())));
    common(&host).height.set(Some(Size::Pixels(480.into())));
    let viewport = CoverageRect {
        left: 0.,
        top: 0.,
        right: 640.,
        bottom: 480.,
    };
    let mut activity = ScrollActivityTracker::default();
    let mut visible = Vec::new();
    for (now, x) in [(0., 320.), (16., 620.)] {
        root.own_camera_view
            .set(Some(crate::camera::resolve_for_test(
                root.spatial_geometry.get(),
                [x, 240., 0.],
                [0.; 3],
                1.,
            )));
        engine.tick();
        let tab = host.presentation_transform_and_bounds.get();
        let m = tab.transform.coeffs();
        let bounds = CoverageRect {
            left: m[4],
            top: m[5],
            right: m[4] + tab.bounds.0,
            bottom: m[5] + tab.bounds.1,
        };
        activity.observe(host.id.to_u32(), [0., 0.], now, &format!("{m:?}"));
        let hosts = BTreeMap::from([(
            host.id.to_u32(),
            PreparationHost {
                geometry: HostObservation {
                    id: host.id.to_u32(),
                    parent: None,
                    bounds,
                    visible: viewport,
                    viewport: [tab.bounds.0, tab.bounds.1],
                    offset: [0., 0.],
                    basis: [m[0], m[1], m[2], m[3]],
                    projection: None,
                    active: true,
                },
                content: [1000., 480.],
                intermediate_clip: None,
            },
        )]);
        let plans = prepare_hosts(
            &hosts,
            viewport,
            &activity,
            now,
            PreparationPolicy::default(),
        )
        .unwrap();
        visible.push(plans[&host.id.to_u32()].required);
        assert_eq!(activity.get(host.id.to_u32(), now).speed, [0., 0.]);
    }
    close(visible[0].left, 0.);
    close(visible[0].right, 640.);
    close(visible[1].left, 300.);
    close(visible[1].right, 940.);
}

#[test]
fn camera_inline_timeline_samples_the_root_clock() {
    use pax_manifest::{
        SettingElement, TimelineKeyframe, TimelineMarker, TimelineTrackDefinition,
        TimelineTrackElement, Token, ValueDefinition,
    };
    let mut declaration = camera(&[]);
    declaration.elements.push(SettingElement::Setting(
        Token::new_without_location("zoom".into()),
        ValueDefinition::Timeline(TimelineTrackDefinition {
            elements: [(0, 2), (2, 4)]
                .into_iter()
                .map(|(frame, zoom)| {
                    TimelineTrackElement::Keyframe(TimelineKeyframe {
                        marker: TimelineMarker::Frame(frame),
                        value: ValueDefinition::LiteralValue(pax_runtime_api::PaxValue::Numeric(
                            zoom.into(),
                        )),
                        easing: None,
                    })
                })
                .collect(),
            repeat: Some(false),
            ..Default::default()
        }),
    ));
    let (engine, _, _, child) = fixture_with_camera(Some(declaration));
    engine.runtime_context.globals().elapsed_frames.set(0);
    close(
        child.presentation_transform_and_bounds.get().transform.m[0],
        2.,
    );
    engine.runtime_context.globals().elapsed_frames.set(1);
    close(
        child.presentation_transform_and_bounds.get().transform.m[0],
        3.,
    );
    engine.runtime_context.globals().elapsed_frames.set(2);
    close(
        child.presentation_transform_and_bounds.get().transform.m[0],
        4.,
    );
}

#[test]
fn perspective_camera_binds_lens_preserves_layout_and_matches_input() {
    let (mut engine, _, _, child) = fixture_with_camera(Some(camera(&[
        ("projection", "CameraMode::Perspective"),
        ("position", "[50%, 50%, 600px]"),
        ("vertical_fov", "60deg"),
        ("near", "10px"),
        ("far", "2000px"),
    ])));
    let before = child.spatial_geometry.get();
    engine
        .runtime_context
        .edit_globals(|g| g.renderer_is_gpu = true);
    common(&child).width.set(Some(Size::Pixels(100.into())));
    common(&child).height.set(Some(Size::Pixels(80.into())));
    common(&child)
        .rotate_y
        .set(Some(Rotation::Degrees(30.into())));
    engine.tick();
    assert!(child.has_perspective_camera());
    let world = child.spatial_geometry.get();
    let plane = child.presentation_projection().unwrap();
    assert!(plane.clips_depth());
    assert!(plane.mapping().as_affine().is_none());
    let screen = plane
        .mapping()
        .project(kurbo::Point::new(25., 30.))
        .unwrap();
    assert!(child.ray_cast_test(pax_runtime_api::math::Point2::new(screen.x, screen.y)));
    let local = child
        .get_node_context(&engine.runtime_context)
        .local_point(pax_runtime_api::math::Point2::new(screen.x, screen.y));
    close(local.x, 0.25);
    close(local.y, 0.375);
    let prepared = engine.runtime_context.canvas_geometry_for_node(&child);
    assert_eq!(prepared.projection, Some(plane));
    assert!(prepared.coverage_bounds.unwrap().width() > 0.);
    assert_eq!(child.spatial_geometry.get(), world);
    assert_eq!(
        before.world_transform().coeffs()[12],
        world.world_transform().coeffs()[12]
    );
}

#[test]
fn perspective_camera_recovers_when_backend_capability_changes() {
    let (mut engine, _, _, child) = fixture_with_camera(Some(camera(&[
        ("projection", "CameraMode::Perspective"),
        ("z", "600px"),
    ])));
    engine
        .runtime_context
        .edit_globals(|g| g.renderer_is_gpu = false);
    engine.tick();
    assert!(
        !child.has_perspective_camera(),
        "CPU must not approximate perspective"
    );
    engine
        .runtime_context
        .edit_globals(|g| g.renderer_is_gpu = true);
    engine.tick();
    assert!(child.has_perspective_camera());
    engine
        .runtime_context
        .edit_globals(|g| g.renderer_is_gpu = false);
    engine.tick();
    assert!(!child.has_perspective_camera());
    assert_eq!(
        child.presentation_transform_and_bounds.get().transform,
        child.transform_and_bounds.get().transform
    );
}

#[test]
fn perspective_camera_invalid_lenses_use_default_view() {
    for (key, value) in [
        ("near", "0px"),
        ("far", "0px"),
        ("vertical_fov", "180deg"),
        ("vertical_fov", "0deg"),
    ] {
        let (mut engine, _, _, child) = fixture_with_camera(Some(camera(&[
            ("projection", "CameraMode::Perspective"),
            ("z", "600px"),
            (key, value),
        ])));
        engine
            .runtime_context
            .edit_globals(|g| g.renderer_is_gpu = true);
        engine.tick();
        assert!(!child.has_perspective_camera(), "{key}={value}");
        assert_eq!(
            child.presentation_transform_and_bounds.get().transform,
            child.transform_and_bounds.get().transform
        );
    }
}

#[test]
fn perspective_nested_scroll_projects_world_motion_for_native_and_canvas_planes() {
    use kurbo::{Affine, Point};
    use pax_runtime_api::math::{PlanarProjection, ProjectedPlane};
    let scroller = |children| -> Rc<dyn InstanceNode> {
        Rc::new(Surface(
            BaseInstance::new(
                args(children),
                InstanceFlags {
                    layer: Layer::Canvas,
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    is_component: false,
                    is_slot: false,
                },
            ),
            true,
        ))
    };
    let leaf = Surface::instantiate(args(vec![]));
    let inner = scroller(vec![leaf]);
    let outer = scroller(vec![inner]);
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![outer]));
    root_args.component_camera = Some(camera(&[
        ("projection", "CameraMode::Perspective"),
        ("z", "600px"),
        ("vertical_fov", "60deg"),
    ]));
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.runtime_context.resize_canvas_layers_to(3);
    engine.tick();
    let outer = root.children.get()[0].clone();
    let inner = outer.children.get()[0].clone();
    let leaf = inner.children.get()[0].clone();
    common(&outer)
        .rotate_x
        .set(Some(Rotation::Degrees(15.into())));
    common(&inner)
        .rotate_y
        .set(Some(Rotation::Degrees(20.into())));
    common(&leaf).z.set(Some(120.0.into()));
    common(&leaf)
        .rotate_y
        .set(Some(Rotation::Degrees(30.into())));
    engine.tick();
    let ctx = &engine.runtime_context;
    let layout = leaf.spatial_geometry.get();
    let mut occlusion = leaf.occlusion.get();
    occlusion.render_layer_id = 2;
    leaf.occlusion.set(occlusion);
    ctx.register_layer_scroller_owner(2, inner.id);

    let native_mapping = |node: &ExpandedNode| {
        let m = ctx.native_presentation_matrix_for_node(node);
        if m.len() == 6 {
            PlanarProjection::from(Affine::new(m.try_into().unwrap()))
        } else {
            ProjectedPlane::new(Transform3::<NodeLocal>::new(m.try_into().unwrap()), false)
                .unwrap()
                .mapping()
        }
    };
    for offsets in [
        [[0., 0.], [0., 0.]],
        [[30., 70.], [10., 20.]],
        [[3., 4.], [40., 50.]],
    ] {
        for (node, [x, y]) in [&outer, &inner].into_iter().zip(offsets) {
            ctx.set_scroller_surface_state(
                node.id.to_u32(),
                crate::ScrollerSurfaceState {
                    scroll_x: x,
                    scroll_y: y,
                    presentation_scroll_x: x,
                    presentation_scroll_y: y,
                    viewport_width: 200.,
                    viewport_height: 150.,
                    content_width: 1000.,
                    content_height: 1000.,
                    ..Default::default()
                },
            );
        }
        // Reproduce the host's actual nesting: each leaf transform is followed
        // by scroll, then its inverse on the native-content overlay.
        let mut native_host = PlanarProjection::default();
        for (node, [x, y]) in [&outer, &inner].into_iter().zip(offsets) {
            let delivered = native_mapping(node);
            native_host = native_host
                * delivered
                * PlanarProjection::from(Affine::translate((-x, -y)))
                * delivered.inverse().unwrap();
        }
        let native = native_host * native_mapping(&leaf);
        let canvas = ctx.canvas_surface_projection_for_node(&leaf).unwrap();
        let inner_global = native_mapping(&outer)
            * PlanarProjection::from(Affine::translate((-offsets[0][0], -offsets[0][1])))
            * native_mapping(&outer).inverse().unwrap()
            * native_mapping(&inner);
        let canvas_presented = inner_global
            * PlanarProjection::from(Affine::translate((-offsets[1][0], -offsets[1][1])))
            * canvas.mapping();

        let mut displacement = [0.; 3];
        for (node, [x, y]) in [&outer, &inner].into_iter().zip(offsets) {
            let world = node.spatial_geometry.get().world_transform().coeffs();
            for axis in 0..3 {
                displacement[axis] -= world[axis] * x + world[4 + axis] * y;
            }
        }
        let expected = leaf
            .presentation_view
            .get()
            .plane(SpatialGeometry::new(
                Transform3::<Window>::translate(Vector3::new(
                    displacement[0],
                    displacement[1],
                    displacement[2],
                )) * layout.world_transform(),
                layout.bounds,
            ))
            .unwrap();
        for local in [Point::new(20., 30.), Point::new(80., 60.)] {
            let expected_point = expected.mapping().project(local).unwrap();
            for actual in [native, canvas_presented] {
                let actual = actual.project(local).unwrap();
                close(actual.x, expected_point.x);
                close(actual.y, expected_point.y);
            }
            let picked = expected.local_point(expected_point).unwrap();
            close(picked.x, local.x);
            close(picked.y, local.y);
        }
        assert!(ctx.is_canvas_node_dirty(&leaf.id));
        assert_eq!(leaf.spatial_geometry.get(), layout);
    }
}

// Exercise native controls without canvas paint bounds and custom primitives
// whose occlusion paths are known but whose paint bounds are unbounded.
struct OcclusionSurface(BaseInstance);
impl OcclusionSurface {
    fn new(layer: Layer) -> Rc<Self> {
        Rc::new(Self(BaseInstance::new(
            args(vec![]),
            InstanceFlags {
                layer,
                invisible_to_slot: false,
                invisible_to_raycasting: false,
                is_component: false,
                is_slot: false,
            },
        )))
    }
}
impl InstanceNode for OcclusionSurface {
    fn instantiate(_: InstantiationArgs) -> Rc<Self> {
        unreachable!()
    }
    fn base(&self) -> &BaseInstance {
        &self.0
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("OcclusionSurface").finish()
    }
    fn resolve_local_coverage_path(&self, node: &ExpandedNode) -> Option<kurbo::BezPath> {
        use kurbo::Shape;
        let (w, h) = node.spatial_geometry.get().bounds;
        (self.0.flags().layer == Layer::Canvas).then(|| kurbo::Rect::new(0., 0., w, h).to_path(0.1))
    }
}

#[test]
fn perspective_native_masks_split_both_crossing_surfaces_and_follow_movement() {
    use kurbo::Shape;
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![
        OcclusionSurface::new(Layer::Native),
        OcclusionSurface::new(Layer::Canvas),
        OcclusionSurface::new(Layer::Native),
        OcclusionSurface::new(Layer::Canvas),
    ]));
    root_args.component_camera = Some(camera(&[
        ("projection", "CameraMode::Perspective"),
        ("z", "600px"),
    ]));
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    engine
        .runtime_context
        .edit_globals(|g| g.renderer_is_gpu = true);
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    let nodes = root.children.get();
    for (i, node) in nodes.iter().enumerate() {
        let cp = common(node);
        cp.x.set(Some(Size::Pixels(320.into())));
        cp.y.set(Some(Size::Pixels(240.into())));
        cp.anchor_x.set(Some(Size::Percent(50.into())));
        cp.anchor_y.set(Some(Size::Percent(50.into())));
        cp.width.set(Some(Size::Pixels(240.into())));
        cp.height.set(Some(Size::Pixels(100.into())));
        cp.rotate_y
            .set(Some(Rotation::Degrees(if i >= 2 { 45 } else { 0 }.into())));
    }
    engine.runtime_context.take_native_messages();
    let messages = engine.tick();
    for (index, covered_x, visible_x) in [(0, 70., 170.), (2, 170., 70.)] {
        let patch = messages
            .iter()
            .rev()
            .find_map(|m| match m {
                pax_message::NativeMessage::NativeMaskUpdate(p)
                    if p.id == nodes[index].id.to_u32() =>
                {
                    Some(p)
                }
                _ => None,
            })
            .expect("crossing native leaf must receive a mask");
        let covered = |x| {
            patch.entries.iter().any(|entry| {
                let point = kurbo::Point::new(x, 50.);
                kurbo::BezPath::from_svg(&entry.path)
                    .unwrap()
                    .contains(point)
                    && entry
                        .clips
                        .iter()
                        .all(|clip| kurbo::BezPath::from_svg(clip).unwrap().contains(point))
            })
        };
        assert!(
            covered(covered_x),
            "surface {index}: nearer card must hide text"
        );
        assert!(
            !covered(visible_x),
            "surface {index}: nearer text must remain visible"
        );
    }
    let crossing_hash = nodes[0].native_mask_hash.get();
    let rebuilds = engine.runtime_context.occlusion_stats().rebuilds;
    for z in [-200., 0.] {
        common(&nodes[3])
            .z
            .set(Some(pax_runtime_api::Depth::from(z)));
        engine.tick();
        assert_eq!(engine.runtime_context.occlusion_stats().rebuilds, rebuilds);
        assert_eq!(
            nodes[0].native_mask_hash.get(),
            if z < 0. { 0 } else { crossing_hash }
        );
    }
    // Move the native leaf first, then move only the occluder. Both the native
    // index and the old/new paint envelopes must use projected surface space.
    common(&nodes[0]).x.set(Some(Size::Pixels(400.into())));
    engine.tick();
    common(&nodes[3])
        .z
        .set(Some(pax_runtime_api::Depth::from(200.)));
    engine.tick();
    let incremental = nodes[0].native_mask_hash.get();
    assert_ne!(incremental, 0);
    engine.runtime_context.mark_occlusion_dirty();
    engine.tick();
    assert_eq!(nodes[0].native_mask_hash.get(), incremental);
}

struct SolidSurface(BaseInstance, Rc<pax_runtime_api::mesh3::ConvexMesh>);
impl InstanceNode for SolidSurface {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self(
            BaseInstance::new(
                args,
                InstanceFlags {
                    layer: Layer::Canvas,
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    is_component: false,
                    is_slot: false,
                },
            ),
            pax_runtime_api::mesh3::ConvexMesh::sphere(24, 12),
        ))
    }
    fn base(&self) -> &BaseInstance {
        &self.0
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("SolidSurface").finish()
    }
    fn resolve_convex_mesh(
        &self,
        node: &ExpandedNode,
    ) -> Option<Rc<pax_runtime_api::mesh3::PosedConvexMesh>> {
        let local: Transform3<NodeLocal> =
            Transform3::<NodeLocal>::translate(Vector3::new(120., 50., 0.))
                * Transform3::scale(Vector3::new(90., 70., 100.));
        self.1
            .posed((node.spatial_geometry.get().world_transform() * local).coeffs())
    }
    fn resolve_coverage_path(&self, node: &ExpandedNode) -> Option<kurbo::BezPath> {
        let solid = node.project_convex_mesh(self.resolve_convex_mesh(node)?.as_ref())?;
        let mut p = kurbo::BezPath::new();
        for (i, v) in solid.silhouette().iter().enumerate() {
            if i == 0 {
                p.move_to((v[0], v[1]));
            } else {
                p.line_to((v[0], v[1]));
            }
        }
        if !p.elements().is_empty() {
            p.close_path();
        }
        Some(p)
    }
}

#[test]
fn solid_camera_picking_native_masks_and_scroll_charts_agree() {
    use kurbo::{Affine, Point, Shape};
    for (perspective, opacity) in [(false, 1.), (true, 1.), (false, 0.35), (true, 0.35)] {
        let native = OcclusionSurface::new(Layer::Native);
        let solid = SolidSurface::instantiate(args(vec![]));
        let host = Rc::new(Surface(
            BaseInstance::new(
                args(vec![native, solid]),
                InstanceFlags {
                    layer: Layer::Canvas,
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    is_component: false,
                    is_slot: false,
                },
            ),
            true,
        ));
        let mut root_args = args(vec![]);
        root_args.component_template = Some(RefCell::new(vec![host]));
        root_args.component_camera = Some(camera(if perspective {
            &[
                ("projection", "CameraMode::Perspective"),
                ("z", "600px"),
                ("near", "380px"),
            ]
        } else {
            &[("zoom", "1.3"), ("x", "300px"), ("rotate_y", "12deg")]
        }));
        let mut engine = crate::test_support::empty_engine(
            (640., 480.),
            Platform::Web,
            OS::Mac,
            Box::new(|| 0),
            Default::default(),
        );
        engine.runtime_context.edit_globals(|g| {
            g.renderer_is_gpu = true;
            g.browser_allows_scroller_vector_layers.set(true);
        });
        let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
        engine.tick();
        let host = root.children.get()[0].clone();
        let native = host.children.get()[0].clone();
        let solid = host.children.get()[1].clone();
        let cp = common(&host);
        cp.opacity.set(Some(opacity.into()));
        cp.x.set(Some(Size::Pixels(100.into())));
        cp.y.set(Some(Size::Pixels(120.into())));
        cp.rotate_y.set(Some(Rotation::Degrees(20.into())));
        for n in [&native, &solid] {
            let cp = common(n);
            cp.width.set(Some(Size::Pixels(240.into())));
            cp.height.set(Some(Size::Pixels(100.into())));
            cp.z.set(Some(100.0.into()));
        }
        for scroll in [0., 30., 60.] {
            let ctx = &engine.runtime_context;
            ctx.set_scroller_surface_state(
                host.id.to_u32(),
                crate::ScrollerSurfaceState {
                    scroll_x: scroll,
                    presentation_scroll_x: scroll,
                    viewport_width: 500.,
                    viewport_height: 300.,
                    content_width: 1000.,
                    content_height: 1000.,
                    ..Default::default()
                },
            );
            // Force an initial mask delivery even if it matches an earlier state.
            native.native_mask_hash.set(u64::MAX);
            ctx.mark_node_occlusion_dirty(native.id.to_u32());
            let messages = engine.tick();
            let ctx = &engine.runtime_context;
            let prepared = ctx.canvas_geometry_for_node(&solid);
            assert_eq!(ctx.get_layer_scroller_owner(prepared.layer), Some(host.id));
            let mesh = prepared.projected_solid.as_ref().unwrap();
            let native_projection = ctx.presented_projection_for_node(&native).unwrap();
            let plane = native_projection.depth_plane().unwrap();
            let scroll_t = ctx.presentation_scroll_transform_for_node(&solid);
            let offset = if perspective {
                0.
            } else {
                ctx.presentation_depth_offset(&solid)
            };
            let patch = messages
                .iter()
                .rev()
                .find_map(|m| match m {
                    pax_message::NativeMessage::NativeMaskUpdate(p)
                        if p.id == native.id.to_u32() =>
                    {
                        Some(p)
                    }
                    _ => None,
                })
                .unwrap();
            assert!(patch.entries.iter().all(|entry| entry.opacity == Some(1.)),
                "shared fades must not reveal hidden native content: perspective={perspective}, opacity={opacity}");
            let mut covered = 0;
            let mut visible = 0;
            for y in (10..90).step_by(13) {
                for x in (10..230).step_by(17) {
                    let local = Point::new(x as f64, y as f64);
                    if !native_projection.contains_local(local) {
                        continue;
                    }
                    let screen = native_projection.mapping().project(local).unwrap();
                    let p = if perspective {
                        screen
                    } else {
                        scroll_t.inverse() * screen
                    };
                    let depth = mesh.depth_at(p.x, p.y).map(|d| d + offset);
                    let expected =
                        depth.is_some_and(|d| d > plane.depth_at(screen.x, screen.y) + 1e-7);
                    let masked = patch.entries.iter().any(|entry| {
                        kurbo::BezPath::from_svg(&entry.path)
                            .unwrap()
                            .contains(local)
                            && entry
                                .clips
                                .iter()
                                .all(|clip| kurbo::BezPath::from_svg(clip).unwrap().contains(local))
                    });
                    assert_eq!(
                        masked, expected,
                        "perspective={perspective} scroll={scroll} local={local:?}"
                    );
                    let hits = ctx.get_elements_beneath_ray(
                        None,
                        pax_runtime_api::math::Point2::new(screen.x, screen.y),
                        false,
                        vec![],
                        false,
                    );
                    assert_eq!(hits.iter().any(|n| n.id == solid.id), depth.is_some());
                    if expected {
                        covered += 1;
                    } else {
                        visible += 1;
                    }
                }
            }
            assert!(covered > 0 && visible > 0);
            let chart = prepared.solid_projection.unwrap();
            let chart_mesh = mesh.mesh.projected(chart, None);
            let canvas_to_window = if perspective {
                ctx.window_to_canvas_mapping(prepared.layer)
                    .unwrap()
                    .inverse()
                    .unwrap()
            } else {
                pax_runtime_api::math::PlanarProjection::from(Affine::from(
                    host.presentation_transform_and_bounds.get().transform,
                ))
            };
            let center = chart_mesh.coverage_bounds().unwrap().center();
            let window = canvas_to_window.project(center).unwrap();
            let w = canvas_to_window.homogeneous(center)[2];
            close(
                chart_mesh.depth_at(center.x, center.y).unwrap() / w,
                mesh.depth_at(window.x, window.y).unwrap(),
            );
        }
        let mask = native.native_mask_hash.get();
        assert_ne!(mask, 0);
        common(&solid).z.set(Some((-20000.0).into()));
        engine.tick();
        assert_eq!(
            native.native_mask_hash.get(),
            0,
            "hidden solid must release its old native mask"
        );
        common(&solid).z.set(Some(100.0.into()));
        engine.tick();
        assert_eq!(
            native.native_mask_hash.get(),
            mask,
            "returning solid must recover its mask"
        );
    }
}
