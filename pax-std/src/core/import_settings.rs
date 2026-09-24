use std::iter;
use std::rc::Rc;

use pax_engine::api::Duration;
use pax_engine::{pax, Property};
use pax_runtime::api::{borrow, use_RefCell, Layer};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

use_RefCell!();

/// Mounts a non-rendering subtree whose component descendants export selector settings.
/// `transition=SettingsTransition::Ease(400ms, TransitionCurve::InOutQuad)`
/// eases changes to effective settings after the initial appearance. Explicit
/// inline values, property timelines, and two-way bindings retain ownership.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::core::import_settings::ImportSettingsInstance")]
pub struct ImportSettings {
    /// Optional motion for changes to effective imported settings. Each new
    /// target starts a full-duration transition from the currently displayed value.
    pub transition: Property<SettingsTransition>,
}

/// Motion shared by the settings exported from one ImportSettings instance.
/// Named timeline playback is intentionally not supported yet.
#[pax]
#[engine_import_path("pax_engine")]
pub enum SettingsTransition {
    /// Apply settings immediately (the default).
    #[default]
    None,
    /// Interpolate changed settings using this duration and easing curve.
    /// Nonpositive or non-finite durations apply the destination immediately.
    Ease(Duration, TransitionCurve),
}

/// Built-in easing curves for automatic settings transitions.
#[pax]
#[engine_import_path("pax_engine")]
pub enum TransitionCurve {
    /// Constant progress.
    #[default]
    Linear,
    /// Retain the source until the duration ends.
    Hold,
    /// Accelerate from rest.
    InQuad,
    /// Decelerate into the destination.
    OutQuad,
    /// Accelerate, then decelerate.
    InOutQuad,
    /// Move back before accelerating forward.
    InBack,
    /// Overshoot, then settle.
    OutBack,
    /// Anticipation and overshoot.
    InOutBack,
}

impl TransitionCurve {
    fn name(&self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::Hold => "Hold",
            Self::InQuad => "InQuad",
            Self::OutQuad => "OutQuad",
            Self::InOutQuad => "InOutQuad",
            Self::InBack => "InBack",
            Self::OutBack => "OutBack",
            Self::InOutBack => "InOutBack",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_engine::api::{CoercionRules, ToPaxValue};

    #[test]
    fn transition_policy_round_trips_through_typed_values() {
        let policy = SettingsTransition::Ease(
            Duration::Milliseconds(400.into()),
            TransitionCurve::InOutQuad,
        );
        let SettingsTransition::Ease(duration, curve) =
            SettingsTransition::try_coerce(policy.to_pax_value()).unwrap()
        else {
            panic!("expected enabled policy");
        };
        assert_eq!(duration, Duration::Milliseconds(400.into()));
        assert_eq!(curve.name(), "InOutQuad");
    }
}

pub struct ImportSettingsInstance {
    base: BaseInstance,
}

impl InstanceNode for ImportSettingsInstance {
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

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let transition =
            expanded_node.with_properties_unwrapped(|properties: &mut ImportSettings| {
                properties.transition.clone()
            });
        let dependencies = [transition.untyped()];
        *expanded_node.import_settings_transition.borrow_mut() =
            Some(Property::computed_with_name(
                move || match transition.get() {
                    SettingsTransition::None => None,
                    SettingsTransition::Ease(duration, curve) => {
                        Some(pax_runtime::SettingsTransitionConfig {
                            duration,
                            curve: curve.name(),
                        })
                    }
                },
                &dependencies,
                "imported settings transition",
            ));
        let env = Rc::clone(&expanded_node.stack);
        let children = borrow!(self.base().get_instance_children());
        let sidecar_children = expanded_node.create_children_detached(
            children.iter().cloned().zip(iter::repeat(env)),
            context,
            &Rc::downgrade(expanded_node),
        );
        let sidecar_children = expanded_node.attach_sidecar_children(
            sidecar_children,
            context,
            &expanded_node.parent_frame,
        );
        for child in sidecar_children.iter() {
            child.recurse_control_flow_expansion(context);
        }
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let sidecar_children = std::mem::take(&mut *expanded_node.sidecar_children.borrow_mut());
        for child in sidecar_children {
            child.recurse_unmount(context);
        }
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("ImportSettings").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
