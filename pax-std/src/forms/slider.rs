use pax_engine::api::LocalProperty;
use pax_message::{AnyCreatePatch, NativeInterrupt, SliderPatch};
use pax_runtime::api::{use_RefCell, Layer, Property};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

use_RefCell!();
use pax_runtime::api::*;

use pax_engine::pax;
use std::collections::HashMap;
use std::rc::Rc;

use crate::common::{native_surface_opacity, patch_if_needed, patch_liquid_glass_if_needed};

/// A slider control, delegating to a platform-specific native range input.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::forms::slider::SliderInstance")]
#[custom(Default)]
pub struct Slider {
    /// Track background color.
    pub background: Property<Color>,
    /// Accent color for the active track/thumb, when supported.
    pub accent: Property<Color>,
    /// Slider corner radius, in pixels.
    pub corner_radius: Property<f64>,
    /// Current slider value.
    pub value: Property<f64>,
    /// Step interval.
    pub step: Property<f64>,
    /// Minimum value.
    pub min: Property<f64>,
    /// Maximum value.
    pub max: Property<f64>,
}

impl Default for Slider {
    fn default() -> Self {
        Self {
            value: Property::new(0.0),
            step: Property::new(1.0),
            min: Property::new(0.0),
            max: Property::new(100.0),
            accent: Property::new(Color::rgb(27.into(), 100.into(), 242.into())),
            corner_radius: Property::new(5.0),
            background: Property::new(Color::rgb(229.into(), 231.into(), 235.into())),
        }
    }
}

// Runtime instance backing `<Slider>`.
pub struct SliderInstance {
    base: BaseInstance,
    native_values: Rc<RefCell<SliderNativeValues>>,
}

fn slider_values_equal(lhs: f64, rhs: f64) -> bool {
    (lhs - rhs).abs() <= 0.000001
}

// Repeated template nodes share an InstanceNode. Only the expanded node that
// supplied native input may suppress its corresponding outgoing value patch.
#[derive(Default)]
struct SliderNativeValues {
    pending: HashMap<u32, f64>,
}

impl SliderNativeValues {
    fn record(&mut self, id: u32, value: f64) {
        self.pending.insert(id, value);
    }

    fn patch_value(
        &mut self,
        id: u32,
        value: f64,
        previous: &mut Option<f64>,
        patch: &mut Option<f64>,
    ) -> bool {
        match self.pending.remove(&id) {
            Some(native) if slider_values_equal(native, value) => {
                *previous = Some(value);
                false
            }
            Some(_) => {
                // A handler can reject input and restore the previous engine value.
                // The native widget still needs that correction even if our cached
                // engine value did not change.
                *previous = Some(value);
                *patch = Some(value);
                true
            }
            None => patch_if_needed(previous, patch, value),
        }
    }

    fn unmount(&mut self, id: u32) {
        self.pending.remove(&id);
    }
}

impl InstanceNode for SliderInstance {
    fn property_requires_input_recompute(&self, _property_name: &str) -> bool {
        false
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
                    invisible_to_raycasting: false,
                    layer: Layer::Native,
                    is_component: false,
                    is_slot: false,
                },
            ),
            native_values: Rc::new(RefCell::new(SliderNativeValues::default())),
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        // Send creation message
        let id = expanded_node.id.clone();
        context.enqueue_native_message(pax_message::NativeMessage::SliderCreate(AnyCreatePatch {
            id: id.to_u32(),
            parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
            render_layer_id: 0,
        }));

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(SliderPatch {
            id: id.to_u32(),
            ..Default::default()
        }));
        let native_values = self.native_values.clone();

        let deps: Vec<_> = borrow_mut!(expanded_node.properties_scope)
            .values()
            .cloned()
            .map(|v| v.get_untyped_property().clone())
            .chain([
                crate::common::native_geometry_dependency(expanded_node),
                expanded_node.computed_opacity.untyped(),
                expanded_node.occlusion.untyped(),
                expanded_node.liquid_glass_scope.untyped(),
            ])
            .collect();
        expanded_node
            .changed_listener
            .replace_with(LocalProperty::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        return;
                    };
                    let id = expanded_node.id.clone();
                    let mut old_state = borrow_mut!(last_patch);

                    let mut patch = SliderPatch {
                        id: id.to_u32(),
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut Slider| {
                        let computed_tab = expanded_node.presentation_transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let current_value = properties.value.local().get();
                        let value_updated = borrow_mut!(native_values).patch_value(
                            id.to_u32(),
                            current_value,
                            &mut old_state.value,
                            &mut patch.value,
                        );
                        let updates = [
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.parent_frame,
                                &mut patch.parent_frame,
                                expanded_node.parent_frame.get().map(|v| v.to_u32()),
                            ),
                            patch_if_needed(
                                &mut old_state.z_index,
                                &mut patch.z_index,
                                expanded_node.occlusion.get().z_index,
                            ),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                context.native_presentation_matrix_for_node(&expanded_node),
                            ),
                            patch_if_needed(
                                &mut old_state.accent,
                                &mut patch.accent,
                                (&properties.accent.local().get()).into(),
                            ),
                            value_updated,
                            patch_if_needed(
                                &mut old_state.step,
                                &mut patch.step,
                                properties.step.local().get(),
                            ),
                            patch_if_needed(
                                &mut old_state.min,
                                &mut patch.min,
                                properties.min.local().get(),
                            ),
                            patch_if_needed(
                                &mut old_state.max,
                                &mut patch.max,
                                properties.max.local().get(),
                            ),
                            patch_if_needed(
                                &mut old_state.corner_radius,
                                &mut patch.corner_radius,
                                properties.corner_radius.local().get(),
                            ),
                            patch_if_needed(
                                &mut old_state.background,
                                &mut patch.background,
                                (&properties.background.local().get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.opacity,
                                &mut patch.opacity,
                                native_surface_opacity(&expanded_node, &context),
                            ),
                            patch_liquid_glass_if_needed(
                                &mut old_state.liquid_glass,
                                &mut patch.liquid_glass,
                                &expanded_node,
                            ),
                        ];
                        if updates.into_iter().any(|v| v == true) {
                            context.enqueue_native_message(
                                pax_message::NativeMessage::SliderUpdate(patch),
                            );
                        }
                    });
                    ()
                },
                &deps,
            ));
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.clone();
        borrow_mut!(self.native_values).unmount(id.to_u32());
        expanded_node
            .changed_listener
            .replace_with(LocalProperty::default());
        context.enqueue_native_message(pax_message::NativeMessage::SliderDelete(id.to_u32()));
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Slider").finish_non_exhaustive()
    }

    fn property_requires_occlusion_recompute(&self, _property_name: &str) -> bool {
        false
    }

    fn handle_native_interrupt(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        interrupt: &NativeInterrupt,
    ) {
        if let NativeInterrupt::FormSliderChange(args) = interrupt {
            borrow_mut!(self.native_values).record(expanded_node.id.to_u32(), args.value);
            expanded_node
                .with_properties_unwrapped(|props: &mut Slider| props.value.set(args.value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SliderNativeValues;

    #[test]
    fn repeated_slider_cannot_consume_siblings_acknowledgement() {
        let mut values = SliderNativeValues::default();
        values.record(7, 0.75);
        let mut peer_previous = Some(0.5);
        let mut origin_previous = Some(0.5);
        let mut peer_patch = None;
        let mut origin_patch = None;
        assert!(values.patch_value(8, 0.75, &mut peer_previous, &mut peer_patch));
        assert_eq!(peer_patch, Some(0.75));
        assert!(!values.patch_value(7, 0.75, &mut origin_previous, &mut origin_patch));
        assert_eq!(origin_patch, None);
        assert_eq!(origin_previous, Some(0.75));
    }

    #[test]
    fn application_correction_does_not_leave_a_future_false_acknowledgement() {
        let mut values = SliderNativeValues::default();
        values.record(7, 0.75);
        let mut previous = Some(0.5);
        let mut patch = None;
        assert!(values.patch_value(7, 0.5, &mut previous, &mut patch));
        assert_eq!(patch, Some(0.5), "rejected input must restore native value");
        patch = None;
        assert!(values.patch_value(7, 0.75, &mut previous, &mut patch));
        assert_eq!(
            patch,
            Some(0.75),
            "later updates must not consume a stale ack"
        );
    }

    #[test]
    fn latest_native_value_and_unmount_are_independent_per_node() {
        let mut values = SliderNativeValues::default();
        values.record(7, 0.25);
        values.record(7, 0.5);
        values.record(8, 0.75);
        values.unmount(8);
        assert!(!values.patch_value(7, 0.5000001, &mut Some(0.0), &mut None));
        assert!(values.patch_value(8, 0.75, &mut Some(0.0), &mut None));
        assert!(values.pending.is_empty());
    }
}
