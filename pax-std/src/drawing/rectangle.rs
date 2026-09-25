use kurbo::{Affine, RoundedRect, RoundedRectRadii, Shape};
use pax_runtime::{api::Fill, BaseInstance};
use pax_runtime_api::use_RefCell;

use crate::common::begin_bounded_canvas_node;
use pax_runtime::{ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext};

use pax_runtime::api as pax_runtime_api;
use pax_runtime::api::{Layer, Material, RenderContext, Stroke};
use_RefCell!();
use pax_engine::{helpers, pax, CoercionRules, PaxValue, Property};
use pax_manifest::pax_runtime_api::Numeric;
use std::rc::Rc;

/// A 2D vector rectangle, which covers its bounding box with the specified fill and stroke.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::rectangle::RectangleInstance")]
pub struct Rectangle {
    /// Stroke drawn around the rectangle.
    pub stroke: Property<Stroke>,
    /// Fill painted inside the rectangle.
    pub fill: Property<Fill>,
    /// Light-reactive surface response.
    pub material: Property<Material>,
    /// Per-corner radii.
    pub corner_radius: Property<CornerRadii>,
}

// Runtime instance backing `<Rectangle>`.
pub struct RectangleInstance {
    base: BaseInstance,
}

impl InstanceNode for RectangleInstance {
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
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let tab = expanded_node.transform_and_bounds.clone();
        let (corner_radius, stroke, fill, material) =
            expanded_node.with_properties_unwrapped(|properties: &mut Rectangle| {
                (
                    properties.corner_radius.clone(),
                    properties.stroke.clone(),
                    properties.fill.clone(),
                    properties.material.clone(),
                )
            });

        let deps = &[
            tab.untyped(),
            corner_radius.untyped(),
            stroke.untyped(),
            fill.untyped(),
            material.untyped(),
            expanded_node.computed_opacity.untyped(),
            expanded_node.computed_opacity_scopes.untyped(),
        ];
        let cloned_expanded_node = expanded_node.clone();
        let cloned_context = context.clone();

        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    cloned_context.mark_canvas_node_dirty(cloned_expanded_node.id);
                    cloned_context
                        .set_canvas_dirty(cloned_expanded_node.occlusion.get().render_layer_id)
                },
                deps,
            ));
    }

    fn resolve_coverage_path(&self, expanded_node: &ExpandedNode) -> Option<kurbo::BezPath> {
        expanded_node.with_properties_unwrapped(|properties: &mut Rectangle| {
            let tab = expanded_node.transform_and_bounds.get();
            let (width, height) = tab.bounds;
            let rect = RoundedRect::new(0.0, 0.0, width, height, &properties.corner_radius.get());
            Some(Affine::from(tab.transform) * rect.to_path(0.1))
        })
    }

    fn resolve_alpha_mask_paints(
        &self,
        node: &ExpandedNode,
    ) -> Vec<pax_runtime_api::AlphaMaskPaint> {
        node.with_properties_unwrapped(|p: &mut Rectangle| {
            let (w, h) = node.transform_and_bounds.get().bounds;
            crate::common::alpha_mask_paints(
                node,
                RoundedRect::new(0.0, 0.0, w, h, &p.corner_radius.get()).to_path(0.1),
                p.fill.get(),
                p.stroke.get(),
            )
        })
    }

    fn resolve_coverage_opacity(&self, expanded_node: &ExpandedNode) -> f64 {
        expanded_node.with_properties_unwrapped(|properties: &mut Rectangle| {
            (properties.fill.get().coverage_alpha_0_1() * expanded_node.computed_opacity.get())
                .clamp(0.0, 1.0)
        })
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rc: &mut dyn RenderContext,
    ) {
        if !rtc.is_canvas_node_dirty(&expanded_node.id) {
            return;
        }

        let Some(scope) = begin_bounded_canvas_node(rc, expanded_node, rtc) else {
            return;
        };
        let (width, height) = scope.bounds;

        expanded_node.with_properties_unwrapped(|properties: &mut Rectangle| {
            let rect = RoundedRect::new(0.0, 0.0, width, height, &properties.corner_radius.get());
            let bez_path = rect.to_path(0.1);
            let opacity = scope.paint_opacity;
            let fill = properties.fill.get();
            let stroke = properties.stroke.get();
            let material = properties.material.get();
            rc.save(scope.layer_id);
            rc.transform(scope.layer_id, scope.surface_transform);
            rc.fill_with_material_and_opacity(
                scope.layer_id,
                bez_path.clone(),
                &fill,
                &material,
                opacity,
            );
            //hack to address "phantom stroke" bug on Web
            let width: f64 = stroke.width.get().expect_pixels().to_float();
            if width > f64::EPSILON {
                rc.stroke_with_material_and_opacity(
                    scope.layer_id,
                    bez_path,
                    &stroke,
                    &material,
                    opacity,
                );
            }
            rc.restore(scope.layer_id);
        });
        if rc.end_node(scope.layer_id, scope.node_id) {
            rtc.clear_canvas_node_dirty(&expanded_node.id);
        }
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => expanded_node.with_properties_unwrapped(|r: &mut Rectangle| {
                f.debug_struct("Rectangle")
                    .field("fill", &r.fill.get())
                    .finish()
            }),
            None => f.debug_struct("Rectangle").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}

/// Corner radii, ordered clockwise from top-left.
///
/// Pax templates canonically use a list of one to four values such as
/// `corner_radius=[12, 8, 4]`. Lists expand using the same clockwise arity rules
/// as CSS `border-radius`; a uniform radius may elide the brackets as
/// `corner_radius=12`.
///
/// The zero-based positional ("magic index") contract depends on list arity:
///
/// - `[all]`
/// - `[top-left/bottom-right, top-right/bottom-left]`
/// - `[top-left, top-right/bottom-left, bottom-right]`
/// - `[top-left, top-right, bottom-right, bottom-left]`
///
/// A contextual named object remains available as explicit longhand:
/// `corner_radius={ top_left: 12 top_right: 8 bottom_right: 4 bottom_left: 2 }`.
/// The fully type-qualified constructor also remains valid when explicit type
/// syntax is useful: `corner_radius=CornerRadii { top_left: 12 top_right: 8
/// bottom_right: 4 bottom_left: 2 }`.
/// Interpolation treats each radius independently; zero produces an angular corner.
#[pax]
#[engine_import_path("pax_engine")]
#[has_helpers]
#[custom(CoercionRules, Interpolatable)]
pub struct CornerRadii {
    /// Top-left corner radius.
    pub top_left: Property<Numeric>,
    /// Top-right corner radius.
    pub top_right: Property<Numeric>,
    /// Bottom-right corner radius.
    pub bottom_right: Property<Numeric>,
    /// Bottom-left corner radius.
    pub bottom_left: Property<Numeric>,
}

impl pax_engine::api::Interpolatable for CornerRadii {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self::radii(
            self.top_left.get().interpolate(&other.top_left.get(), t),
            self.top_right.get().interpolate(&other.top_right.get(), t),
            self.bottom_right
                .get()
                .interpolate(&other.bottom_right.get(), t),
            self.bottom_left
                .get()
                .interpolate(&other.bottom_left.get(), t),
        )
    }
}

impl CoercionRules for CornerRadii {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::Vec(values) => Self::from_css_values(values),
            PaxValue::Object(values) => {
                let mut radii = Self::default();
                for (name, value) in values {
                    match name.as_str() {
                        "top_left" => radii.top_left = Property::new(Numeric::try_coerce(value)?),
                        "top_right" => radii.top_right = Property::new(Numeric::try_coerce(value)?),
                        "bottom_right" => {
                            radii.bottom_right = Property::new(Numeric::try_coerce(value)?)
                        }
                        "bottom_left" => {
                            radii.bottom_left = Property::new(Numeric::try_coerce(value)?)
                        }
                        _ => {}
                    }
                }
                Ok(radii)
            }
            PaxValue::Option(value) => match *value {
                Some(value) => Self::try_coerce(value),
                None => Err("None can't be coerced into CornerRadii".to_string()),
            },
            value => {
                let radius = Numeric::try_coerce(value).map_err(|error| {
                    format!("failed to coerce a uniform rectangle corner radius: {error}")
                })?;
                Ok(Self::radii(
                    radius.clone(),
                    radius.clone(),
                    radius.clone(),
                    radius,
                ))
            }
        }
    }
}

impl Into<RoundedRectRadii> for &CornerRadii {
    fn into(self) -> RoundedRectRadii {
        RoundedRectRadii::new(
            self.top_left.get().to_float(),
            self.top_right.get().to_float(),
            self.bottom_right.get().to_float(),
            self.bottom_left.get().to_float(),
        )
    }
}

#[helpers]
impl CornerRadii {
    /// Constructs a `CornerRadii` value from clockwise corner radii.
    pub fn radii(
        top_left: Numeric,
        top_right: Numeric,
        bottom_right: Numeric,
        bottom_left: Numeric,
    ) -> Self {
        CornerRadii {
            top_left: Property::new(top_left),
            top_right: Property::new(top_right),
            bottom_right: Property::new(bottom_right),
            bottom_left: Property::new(bottom_left),
        }
    }

    fn from_css_values(values: Vec<PaxValue>) -> Result<Self, String> {
        if !(1..=4).contains(&values.len()) {
            return Err(format!(
                "rectangle corner radii require 1 to 4 values, got {}",
                values.len()
            ));
        }

        let values = values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                Numeric::try_coerce(value).map_err(|error| {
                    format!("failed to coerce rectangle corner radius {index}: {error}")
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let (top_left, top_right, bottom_right, bottom_left) = match values.as_slice() {
            [all] => (all.clone(), all.clone(), all.clone(), all.clone()),
            [top_left_bottom_right, top_right_bottom_left] => (
                top_left_bottom_right.clone(),
                top_right_bottom_left.clone(),
                top_left_bottom_right.clone(),
                top_right_bottom_left.clone(),
            ),
            [top_left, top_right_bottom_left, bottom_right] => (
                top_left.clone(),
                top_right_bottom_left.clone(),
                bottom_right.clone(),
                top_right_bottom_left.clone(),
            ),
            [top_left, top_right, bottom_right, bottom_left] => (
                top_left.clone(),
                top_right.clone(),
                bottom_right.clone(),
                bottom_left.clone(),
            ),
            _ => unreachable!("corner radius arity was validated above"),
        };

        Ok(Self::radii(top_left, top_right, bottom_right, bottom_left))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn corners_interpolate_independently_through_zero() {
        use pax_engine::api::Interpolatable;
        let from = super::CornerRadii::radii(0.into(), 20.into(), 10.into(), 30.into());
        let to = super::CornerRadii::radii(20.into(), 0.into(), 30.into(), 10.into());
        let mid = from.interpolate(&to, 0.5);
        assert_eq!(mid.top_left.get().to_float(), 10.0);
        assert_eq!(mid.top_right.get().to_float(), 10.0);
        assert_eq!(mid.bottom_right.get().to_float(), 20.0);
        assert_eq!(mid.bottom_left.get().to_float(), 20.0);
        to.top_left.set(100.into());
        assert_eq!(mid.top_left.get().to_float(), 10.0);
    }
    use super::*;

    fn numeric(value: i64) -> PaxValue {
        PaxValue::Numeric(value.into())
    }

    fn assert_radii(radii: CornerRadii, expected: [f64; 4]) {
        assert_eq!(radii.top_left.get().to_float(), expected[0]);
        assert_eq!(radii.top_right.get().to_float(), expected[1]);
        assert_eq!(radii.bottom_right.get().to_float(), expected[2]);
        assert_eq!(radii.bottom_left.get().to_float(), expected[3]);
    }

    #[test]
    fn corner_radius_lists_follow_css_arity() {
        for (values, expected) in [
            (vec![1], [1.0, 1.0, 1.0, 1.0]),
            (vec![1, 2], [1.0, 2.0, 1.0, 2.0]),
            (vec![1, 2, 3], [1.0, 2.0, 3.0, 2.0]),
            (vec![1, 2, 3, 4], [1.0, 2.0, 3.0, 4.0]),
        ] {
            let value = PaxValue::Vec(values.into_iter().map(numeric).collect());
            assert_radii(CornerRadii::try_coerce(value).unwrap(), expected);
        }
    }

    #[test]
    fn bare_corner_radius_is_uniform() {
        assert_radii(CornerRadii::try_coerce(numeric(12)).unwrap(), [12.0; 4]);
    }

    #[test]
    fn object_corner_radius_remains_supported() {
        let value = PaxValue::Object(vec![
            ("top_left".to_string(), numeric(7)),
            ("top_right".to_string(), numeric(6)),
            ("bottom_right".to_string(), numeric(5)),
            ("bottom_left".to_string(), numeric(4)),
            (
                "unknown".to_string(),
                PaxValue::String("ignored".to_string()),
            ),
        ]);

        assert_radii(
            CornerRadii::try_coerce(value).unwrap(),
            [7.0, 6.0, 5.0, 4.0],
        );
    }

    #[test]
    fn corner_radius_lists_reject_invalid_arities() {
        for values in [vec![], vec![1, 2, 3, 4, 5]] {
            let value = PaxValue::Vec(values.into_iter().map(numeric).collect());
            assert!(CornerRadii::try_coerce(value).is_err());
        }
    }
}
