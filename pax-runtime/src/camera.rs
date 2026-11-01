//! Component camera configuration resolves into a view independently of scene layout.
use std::{cell::Cell, rc::Rc};

use pax_manifest::{LiteralBlockDefinition, SettingElement, ValueDefinition};
use pax_runtime_api::{
    camera::{Camera, CameraProjection, CameraSpace},
    math::{ProjectedPlane, Transform3, Vector3},
    properties::PropertyValue,
    Axis, CameraMode, CoercionRules, Depth, Interpolatable, LocalProperty, Numeric, Rotation, Size,
    ToPaxValue, Window,
};

use crate::{
    cartridge::{
        build_common_property_axis_component_value, build_common_property_value, stack_with_base,
    },
    node_interface::NodeLocal,
    ExpandedNode, RuntimePropertiesStackFrame, SpatialGeometry,
};

/// A resolved pixel-homogeneous view, independent of retained layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PresentationView {
    matrix: Transform3<Window, Window>,
    perspective: bool,
    // xyz is a world eye position (w=1) or a direction toward the viewer (w=0).
    pub(crate) lighting_view: [f64; 4],
}
impl Default for PresentationView {
    fn default() -> Self {
        Self {
            matrix: Default::default(),
            perspective: false,
            lighting_view: [0., 0., 1., 0.],
        }
    }
}
impl Interpolatable for PresentationView {}

impl PresentationView {
    pub(crate) fn mesh(self, world: [f64; 16]) -> Option<pax_runtime_api::mesh3::MeshProjection> {
        pax_runtime_api::mesh3::MeshProjection::new(
            (self.matrix * Transform3::<Window, Window>::new(world)).coeffs(),
            self.perspective,
        )
    }
    pub(crate) fn is_perspective(self) -> bool {
        self.perspective
    }

    pub(crate) fn plane(self, geometry: SpatialGeometry) -> Option<ProjectedPlane> {
        ProjectedPlane::new(self.matrix * geometry.world_transform(), self.perspective)
    }

    pub(crate) fn project(self, geometry: SpatialGeometry) -> SpatialGeometry {
        if self == Self::default() {
            geometry
        } else {
            SpatialGeometry::new(self.matrix * geometry.world_transform(), geometry.bounds)
        }
    }
}

fn value<'a>(camera: &'a LiteralBlockDefinition, name: &str) -> Option<&'a ValueDefinition> {
    camera.elements.iter().find_map(|element| match element {
        SettingElement::Setting(key, value) if key.token_value == name => Some(value),
        _ => None,
    })
}

fn bind<T: CoercionRules + PropertyValue + ToPaxValue>(
    camera: &LiteralBlockDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    name: &str,
    default: T,
    axis: Option<usize>,
) -> LocalProperty<Option<T>> {
    let base = LocalProperty::new(default.clone());
    let stack = stack_with_base(stack, base);
    if let Some(value) = value(camera, name) {
        build_common_property_value(name, value, &stack)
    } else if let Some((axis, position)) =
        axis.and_then(|axis| value(camera, "position").map(|v| (axis, v)))
    {
        build_common_property_axis_component_value(name, position, &stack, axis)
    } else {
        LocalProperty::new(Some(default))
    }
}

/// Activate only the application root. Dormant declarations never bind to a runtime scope.
pub(crate) fn bind_root(node: &Rc<ExpandedNode>, gpu: bool) {
    let definition = node.instance_node.borrow().base().component_camera.clone();
    let Some(camera) = definition else {
        node.own_camera_view.replace_with(LocalProperty::new(None));
        return;
    };
    let stack = node.stack.push(node.properties_scope.borrow().clone());
    let x = bind(&camera, &stack, "x", Size::Percent(50.into()), Some(0));
    let y = bind(&camera, &stack, "y", Size::Percent(50.into()), Some(1));
    let z = bind(&camera, &stack, "z", Depth::default(), Some(2));
    let rx = bind(&camera, &stack, "rotate_x", Rotation::default(), None);
    let ry = bind(&camera, &stack, "rotate_y", Rotation::default(), None);
    let rz = bind(&camera, &stack, "rotate", Rotation::default(), None);
    let zoom = bind(&camera, &stack, "zoom", Numeric::from(1), None);
    let projection = bind(
        &camera,
        &stack,
        "projection",
        CameraMode::Orthographic,
        None,
    );
    let vertical_fov = bind(
        &camera,
        &stack,
        "vertical_fov",
        Rotation::Degrees(60.into()),
        None,
    );
    let near = bind(&camera, &stack, "near", Depth(1.into()), None);
    let far = bind(&camera, &stack, "far", Depth(10000.into()), None);
    let world = node.spatial_geometry.clone();
    let deps = [
        x.untyped(),
        y.untyped(),
        z.untyped(),
        rx.untyped(),
        ry.untyped(),
        rz.untyped(),
        zoom.untyped(),
        world.untyped(),
        projection.untyped(),
        vertical_fov.untyped(),
        near.untyped(),
        far.untyped(),
    ];
    let invalid = Cell::new(false);
    let unsupported = Cell::new(false);
    node.own_camera_view.replace_with(LocalProperty::computed(move || {
        let geometry = world.get();
        let bounds = geometry.bounds;
        // Minimized / not-yet-sized viewports do not constitute an authored error.
        if bounds.0 <= 0. || bounds.1 <= 0. { return Some(PresentationView::default()); }
        let mode = projection.get();
        let cpu_perspective = mode == Some(CameraMode::Perspective) && !gpu;
        if unsupported.replace(cpu_perspective) != cpu_perspective && cpu_perspective {
            log::error!("Perspective @camera requires a GPU renderer; CPU perspective is unsupported. Presenting the default orthographic view.");
        }
        if cpu_perspective { return Some(PresentationView::default()); }
        let resolved = (|| {
            resolve_lens(geometry, [x.get()?.evaluate(bounds, Axis::X), y.get()?.evaluate(bounds, Axis::Y), z.get()?.to_float()],
                [rx.get()?.get_as_radians(), ry.get()?.get_as_radians(), rz.get()?.get_as_radians()], zoom.get()?.to_float(),
                mode?, vertical_fov.get()?.get_as_radians(), near.get()?.to_float(), far.get()?.to_float())
        })();
        if invalid.replace(resolved.is_none()) != resolved.is_none() && resolved.is_none() {
            log::warn!("Invalid active @camera: use finite position/rotation, positive finite zoom, 0 < vertical_fov < 180deg and 0 < near < far; presenting the default view until it recovers");
        }
        Some(resolved.unwrap_or_default())
    }, &deps));
}

fn resolve_lens(
    owner: SpatialGeometry,
    position: [f64; 3],
    rotation: [f64; 3],
    zoom: f64,
    projection: CameraMode,
    vertical_fov: f64,
    near: f64,
    far: f64,
) -> Option<PresentationView> {
    if !position
        .iter()
        .chain(rotation.iter())
        .all(|v| v.is_finite())
        || !zoom.is_finite()
        || zoom <= 0.
    {
        return None;
    }
    let center = [owner.bounds.0 / 2., owner.bounds.1 / 2., 0.];
    if projection == CameraMode::Orthographic
        && position == center
        && rotation == [0.; 3]
        && zoom == 1.
    {
        return Some(PresentationView::default());
    }
    type Local = Transform3<NodeLocal, NodeLocal>;
    let pose = Local::translate(Vector3::new(position[0], position[1], position[2]))
        * Local::rotate_z(rotation[2])
        * Local::rotate_y(rotation[1])
        * Local::rotate_x(rotation[0]);
    let world = owner.world_transform();
    let camera_pose = world * pose;
    let pose_matrix = camera_pose.coeffs();
    let direction = [pose_matrix[8], pose_matrix[9], pose_matrix[10]];
    let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !length.is_finite() || length <= 0. {
        return None;
    }
    if projection == CameraMode::Perspective {
        // Zoom scales the focal length; the pose remains independently authored.
        if !vertical_fov.is_finite() || vertical_fov <= 0. || vertical_fov >= std::f64::consts::PI {
            return None;
        }
        let fov = 2. * ((vertical_fov * 0.5).tan() / zoom).atan();
        let lens = CameraProjection::perspective(owner.bounds, fov, near, far)?;
        let view = pose.try_inverse()? * world.try_inverse()?;
        let camera: Camera<Window> =
            Camera::new(Transform3::<Window, CameraSpace>::new(view.coeffs()), lens)?;
        let plane = camera.project_plane(Transform3::<Window, Window>::identity())?;
        let destination = kurbo::Affine::from(owner.orthographic_projection()?.transform);
        return Some(PresentationView {
            matrix: Transform3::new(plane.post_affine(destination).matrix().coeffs()),
            perspective: true,
            lighting_view: [pose_matrix[12], pose_matrix[13], pose_matrix[14], 1.],
        });
    }
    let view = Local::translate(Vector3::new(center[0], center[1], 0.))
        * Local::scale(Vector3::new(zoom, zoom, 1.))
        * pose.try_inverse()?;
    let projected = world * view * world.try_inverse()?;
    projected.is_affine().then_some(PresentationView {
        matrix: projected,
        perspective: false,
        lighting_view: [
            direction[0] / length,
            direction[1] / length,
            direction[2] / length,
            0.,
        ],
    })
}

#[cfg(test)]
pub(crate) fn resolve_for_test(
    owner: SpatialGeometry,
    position: [f64; 3],
    rotation: [f64; 3],
    zoom: f64,
) -> PresentationView {
    resolve_lens(
        owner,
        position,
        rotation,
        zoom,
        CameraMode::Orthographic,
        1.,
        1.,
        10000.,
    )
    .unwrap()
}
