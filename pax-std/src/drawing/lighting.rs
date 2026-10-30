use pax_engine::api::LocalProperty;
use pax_engine::*;
use pax_runtime::api::{
    Color, Depth, Layer, LightShape, Numeric, SceneAmbientLight, SceneLight, SceneLighting, Size,
    Vector3,
};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use std::rc::Rc;

/// A non-rendering container that keeps descendant lights from escaping its subtree.
///
/// Lights outside the frame may still illuminate its descendants. Each expanded
/// frame instance has its own lexical lighting identity.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::lighting::LightFrameInstance")]
pub struct LightFrame {}

pub struct LightFrameInstance {
    base: BaseInstance,
}

impl InstanceNode for LightFrameInstance {
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
                    layer: Layer::DontCare,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }

    fn establishes_light_frame(&self) -> bool {
        true
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("LightFrame").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}

/// A non-rendering light resource that affects light-reactive vector materials in its scene.
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
#[primitive("pax_std::drawing::lighting::LightSourceInstance")]
pub struct LightSource {
    /// Light color.
    pub color: Property<Color>,
    /// Light intensity multiplier.
    pub intensity: Property<f64>,
    /// Point-light attenuation radius in world pixels; ancestor scale does not resize it.
    pub radius: Property<Size>,
    /// Local depth in pixels, transformed with the light and its ancestors.
    pub z: Property<Depth>,
    /// Positional or directional light shape.
    pub shape: Property<LightShape>,
    /// Local direction for directional lights, transformed with the light's ancestors.
    pub direction: Property<Vector3>,
    /// Whether this light contributes to the scene.
    pub enabled: Property<bool>,
}

impl Default for LightSource {
    fn default() -> Self {
        Self {
            color: Property::new(Color::WHITE),
            intensity: Property::new(1.0),
            radius: Property::new(Size::Pixels(Numeric::F64(240.0))),
            z: Property::new(Depth(Numeric::F64(200.0))),
            shape: Property::new(LightShape::Point),
            direction: Property::new(Vector3::new(0.0, 0.0, -1.0)),
            enabled: Property::new(true),
        }
    }
}

pub struct LightSourceInstance {
    base: BaseInstance,
}

impl InstanceNode for LightSourceInstance {
    fn has_scene_lighting(&self) -> bool {
        true
    }

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
        #[cfg(feature = "3d")]
        let geometry = expanded_node.spatial_geometry();
        #[cfg(not(feature = "3d"))]
        let geometry = expanded_node.transform_and_bounds.clone();
        let (color, intensity, radius, z, shape, direction, enabled) = expanded_node
            .with_properties_unwrapped(|properties: &mut LightSource| {
                (
                    properties.color.clone(),
                    properties.intensity.clone(),
                    properties.radius.clone(),
                    properties.z.clone(),
                    properties.shape.clone(),
                    properties.direction.clone(),
                    properties.enabled.clone(),
                )
            });

        let deps = &[
            geometry.untyped(),
            color.untyped(),
            intensity.untyped(),
            radius.untyped(),
            z.untyped(),
            shape.untyped(),
            direction.untyped(),
            enabled.untyped(),
        ];
        let cloned_context = context.clone();

        expanded_node
            .changed_listener
            .replace_with(LocalProperty::computed(
                move || {
                    cloned_context.mark_scene_lighting_dirty();
                },
                deps,
            ));
    }

    fn resolve_scene_light(
        &self,
        expanded_node: &ExpandedNode,
        _context: &RuntimeContext,
    ) -> Option<SceneLight> {
        expanded_node.with_properties_unwrapped(|properties: &mut LightSource| {
            if !properties.enabled.local().get() {
                return None;
            }

            #[cfg(feature = "3d")]
            let geometry = expanded_node.spatial_geometry().get();
            #[cfg(not(feature = "3d"))]
            let geometry = expanded_node.transform_and_bounds.get();
            let (position, direction) = light_pose(
                geometry,
                properties.z.local().get().to_float(),
                properties.direction.local().get(),
            )?;
            Some(SceneLight {
                shape: properties.shape.local().get(),
                position,
                direction,
                color: properties.color.local().get(),
                intensity: properties.intensity.local().get().max(0.0),
                radius: properties
                    .radius
                    .local()
                    .get()
                    .expect_pixels()
                    .to_float()
                    .max(0.0),
                enabled: true,
            })
        })
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("LightSource").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}

/// A non-rendering singleton ambient-light override for its scene.
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
#[primitive("pax_std::drawing::lighting::AmbientLightInstance")]
pub struct AmbientLight {
    /// Ambient color.
    pub color: Property<Color>,
    /// Ambient intensity multiplier.
    pub intensity: Property<f64>,
    /// Whether this ambient override participates in topmost-wins selection.
    pub enabled: Property<bool>,
}

impl Default for AmbientLight {
    fn default() -> Self {
        Self {
            color: Property::new(Color::WHITE),
            intensity: Property::new(SceneLighting::DEFAULT_AMBIENT_INTENSITY),
            enabled: Property::new(true),
        }
    }
}

pub struct AmbientLightInstance {
    base: BaseInstance,
}

impl InstanceNode for AmbientLightInstance {
    fn has_scene_lighting(&self) -> bool {
        true
    }

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
        let (color, intensity, enabled) =
            expanded_node.with_properties_unwrapped(|properties: &mut AmbientLight| {
                (
                    properties.color.clone(),
                    properties.intensity.clone(),
                    properties.enabled.clone(),
                )
            });

        let deps = &[color.untyped(), intensity.untyped(), enabled.untyped()];
        let cloned_context = context.clone();

        expanded_node
            .changed_listener
            .replace_with(LocalProperty::computed(
                move || {
                    cloned_context.mark_scene_lighting_dirty();
                },
                deps,
            ));
    }

    fn resolve_scene_ambient_light(
        &self,
        expanded_node: &ExpandedNode,
        _context: &RuntimeContext,
    ) -> Option<SceneAmbientLight> {
        expanded_node.with_properties_unwrapped(|properties: &mut AmbientLight| {
            properties.enabled.local().get().then(|| SceneAmbientLight {
                color: properties.color.local().get(),
                intensity: properties.intensity.local().get().max(0.0),
            })
        })
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("AmbientLight").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}

// Keep positional and directional lights in the same unprojected frame as materials.
#[cfg(feature = "3d")]
fn light_pose(
    geometry: pax_runtime::SpatialGeometry,
    z: f64,
    direction: Vector3,
) -> Option<(Vector3, Vector3)> {
    let transform = geometry.world_transform();
    if !transform.is_affine() {
        return None;
    }
    let point = transform.project_point(pax_runtime::api::math::Point3::new(
        geometry.bounds.0 * 0.5,
        geometry.bounds.1 * 0.5,
        z,
    ))?;
    let [dx, dy, dz, _] =
        transform.transform_homogeneous([direction.x, direction.y, direction.z, 0.]);
    if ![dx, dy, dz].iter().all(|v| v.is_finite()) {
        return None;
    }
    Some((
        Vector3::new(point.x, point.y, point.z),
        Vector3::new(dx, dy, dz),
    ))
}

#[cfg(not(feature = "3d"))]
fn light_pose(
    geometry: pax_runtime::TransformAndBounds<
        pax_runtime::node_interface::NodeLocal,
        pax_runtime::api::Window,
    >,
    z: f64,
    direction: Vector3,
) -> Option<(Vector3, Vector3)> {
    let m = geometry.transform.m;
    let (x, y) = (geometry.bounds.0 * 0.5, geometry.bounds.1 * 0.5);
    let position = Vector3::new(m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5], z);
    let direction = Vector3::new(
        m[0] * direction.x + m[2] * direction.y,
        m[1] * direction.x + m[3] * direction.y,
        direction.z,
    );
    [
        position.x,
        position.y,
        position.z,
        direction.x,
        direction.y,
        direction.z,
    ]
    .iter()
    .all(|v| v.is_finite())
    .then_some((position, direction))
}

#[cfg(all(test, feature = "3d"))]
mod tests {
    use super::*;
    use pax_runtime::api::{
        math::{Transform3, Vector3 as GeometricVector},
        Window,
    };
    use pax_runtime::{node_interface::NodeLocal, SpatialGeometry};

    #[test]
    fn spatial_light_inherits_parent_pose_and_local_depth() {
        let parent =
            Transform3::<NodeLocal, Window>::translate(GeometricVector::new(100., 200., 300.))
                * Transform3::<NodeLocal>::rotate_y(std::f64::consts::FRAC_PI_2);
        let geometry = SpatialGeometry::new(parent, (20., 40.));
        let (position, direction) = light_pose(geometry, 50., Vector3::new(0., 0., -1.)).unwrap();
        for (a, b) in [
            (position.x, 150.),
            (position.y, 220.),
            (position.z, 290.),
            (direction.x, -1.),
            (direction.y, 0.),
            (direction.z, 0.),
        ] {
            assert!((a - b).abs() < 1e-9, "{a} != {b}");
        }
        assert!(light_pose(geometry, f64::NAN, Vector3::new(0., 0., -1.)).is_none());
    }
}
