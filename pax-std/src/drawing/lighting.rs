use kurbo::Point;
use pax_engine::*;
use pax_runtime::api::{
    Color, Depth, Layer, LightShape, Numeric, SceneAmbientLight, SceneLight, SceneLighting, Size,
    Vector3,
};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use std::rc::Rc;

use crate::common::canvas_surface_transform;

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
    /// Radius for point-light attenuation.
    pub radius: Property<Size>,
    /// Logical scene depth in pixels.
    pub z: Property<Depth>,
    /// Positional or directional light shape.
    pub shape: Property<LightShape>,
    /// Direction for directional lights.
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
        let tab = expanded_node.transform_and_bounds.clone();
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
            tab.untyped(),
            color.untyped(),
            intensity.untyped(),
            radius.untyped(),
            z.untyped(),
            shape.untyped(),
            direction.untyped(),
            enabled.untyped(),
        ];
        let cloned_expanded_node = expanded_node.clone();
        let cloned_context = context.clone();

        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    cloned_context.set_all_canvases_dirty();
                    cloned_context
                        .set_canvas_dirty(cloned_expanded_node.occlusion.get().render_layer_id);
                },
                deps,
            ));
    }

    fn resolve_scene_light(
        &self,
        expanded_node: &ExpandedNode,
        context: &RuntimeContext,
    ) -> Option<SceneLight> {
        expanded_node.with_properties_unwrapped(|properties: &mut LightSource| {
            if !properties.enabled.get() {
                return None;
            }

            let tab = expanded_node.transform_and_bounds.get();
            let center = Point::new(tab.bounds.0 * 0.5, tab.bounds.1 * 0.5);
            let point = canvas_surface_transform(expanded_node, context) * center;
            Some(SceneLight {
                shape: properties.shape.get(),
                position: Vector3::new(point.x, point.y, properties.z.get().to_float()),
                direction: properties.direction.get(),
                color: properties.color.get(),
                intensity: properties.intensity.get().max(0.0),
                radius: properties.radius.get().expect_pixels().to_float().max(0.0),
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
        let cloned_expanded_node = expanded_node.clone();
        let cloned_context = context.clone();

        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    cloned_context.set_all_canvases_dirty();
                    cloned_context
                        .set_canvas_dirty(cloned_expanded_node.occlusion.get().render_layer_id);
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
            properties.enabled.get().then(|| SceneAmbientLight {
                color: properties.color.get(),
                intensity: properties.intensity.get().max(0.0),
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
