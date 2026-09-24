//! Receiver-owned motion for effective imported settings values.

use std::{cell::RefCell, rc::Rc};

use pax_manifest::ValueDefinition;
use pax_runtime_api::{
    properties::{current_frame, register_effect_property, PropertyValue},
    CoercionRules, Duration, EasingCurve, Interpolatable, PaxValue, Property, ToPaxValue,
};

use crate::{
    ExpandedNode, RuntimePropertiesStackFrame, RuntimeResolvedPropertyEntry, RuntimeSettingsSource,
};

/// Runtime projection of an ImportSettings transition; no playback state is shared.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsTransitionConfig {
    pub duration: Duration,
    pub curve: &'static str,
}
impl Default for SettingsTransitionConfig {
    fn default() -> Self {
        Self {
            duration: Duration::default(),
            curve: "Linear",
        }
    }
}
impl Interpolatable for SettingsTransitionConfig {}

#[derive(Clone, Default)]
struct Owner {
    imported: bool,
    explicit: bool,
    config: Option<SettingsTransitionConfig>,
}
impl Interpolatable for Owner {}

fn owner_property(
    entries: &[RuntimeResolvedPropertyEntry],
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Property<Owner> {
    let mut dependencies = Vec::new();
    for entry in entries {
        if let Some(policy) = &entry.transition {
            dependencies.push(policy.untyped());
        }
        if let Some(condition) = &entry.condition {
            crate::cartridge::collect_settings_condition_dependencies(
                condition,
                entry.source_stack.as_ref().unwrap_or(stack),
                &mut dependencies,
            );
        }
    }
    let entries = entries.to_vec();
    let stack = stack.clone();
    Property::computed_with_name(
        move || {
            entries
                .iter()
                .rev()
                .find(|entry| {
                    entry.condition.as_ref().map_or(true, |condition| {
                        crate::cartridge::settings_condition_is_active(
                            condition,
                            entry.source_stack.as_ref().unwrap_or(&stack),
                        )
                    })
                })
                .map(|entry| Owner {
                    imported: matches!(entry.source, RuntimeSettingsSource::ImportedLayer { .. }),
                    explicit: matches!(entry.source, RuntimeSettingsSource::Inline)
                        || matches!(
                            entry.value,
                            ValueDefinition::Timeline(_)
                                | ValueDefinition::Transition(_)
                                | ValueDefinition::DoubleBinding(_)
                        ),
                    config: entry.transition.as_ref().and_then(Property::get),
                })
                .unwrap_or_default()
        },
        &dependencies,
        "settings transition owner",
    )
}

struct Endpoints<T> {
    from: T,
    to: T,
    curve: &'static str,
}

struct SettingsMotion<T: PropertyValue> {
    target: Property<T>,
    owner: Property<Owner>,
    progress: Property<f64>,
    _listener: Property<()>,
}

impl<T: PropertyValue> Drop for SettingsMotion<T> {
    fn drop(&mut self) {
        self.progress.cancel_transitions();
    }
}

fn frozen<T: PropertyValue + CoercionRules + ToPaxValue>(
    value: T,
) -> Result<(T, PaxValue), String> {
    // PaxValue conversion snapshots Property-wrapped leaves. Cloning T alone
    // would retain live provider handles in a transition's endpoints.
    let snapshot = value.to_pax_value();
    T::try_coerce(snapshot.clone()).map(|value| (value, snapshot))
}

impl<T: PropertyValue + CoercionRules + ToPaxValue> SettingsMotion<T> {
    fn new(
        output: &Property<T>,
        target: Property<T>,
        owner: Property<Owner>,
        animate_initial_change: bool,
        interpolate: Rc<dyn Fn(&T, &T, f64) -> T>,
    ) -> Option<Self> {
        let (initial, initial_snapshot) = frozen(output.get()).ok()?;
        let progress = Property::new_with_name(1.0, "settings transition progress");
        let revision = Property::new(0_u64);
        let endpoints = Rc::new(RefCell::new(Endpoints {
            from: initial.clone(),
            to: initial,
            curve: "Linear",
        }));
        let animation = {
            let progress = progress.clone();
            let endpoints = endpoints.clone();
            let dependencies = [progress.untyped(), revision.untyped()];
            Property::computed_with_name(
                move || {
                    let t = progress.get();
                    let endpoints = endpoints.borrow();
                    if t >= 1.0 {
                        endpoints.to.clone()
                    } else {
                        let multiplier =
                            crate::cartridge::easing_curve_from_name(Some(endpoints.curve))
                                .interpolate(&0.0, &1.0, t);
                        interpolate(&endpoints.from, &endpoints.to, multiplier)
                    }
                },
                &dependencies,
                "animated settings value",
            )
        };
        output.cancel_transitions();
        output.replace_with(animation);
        let listener = {
            let target = target.clone();
            let owner = owner.clone();
            let output = output.clone();
            let progress = progress.clone();
            let previous =
                RefCell::new((initial_snapshot, Owner::default(), !animate_initial_change));
            let dependencies = [target.untyped(), owner.untyped()];
            Property::computed_with_name(
                move || {
                    let new_owner = owner.get();
                    let next = target.get();
                    let snapshot = next.clone().to_pax_value();
                    let (next, can_interpolate) = match T::try_coerce(snapshot.clone()) {
                        Ok(frozen) => (frozen, true),
                        // Custom types without a lossless value roundtrip still follow
                        // their target, but cannot retain a safe interpolation endpoint.
                        Err(_) => (next, false),
                    };
                    let mut previous = previous.borrow_mut();
                    let config = if new_owner.explicit {
                        None
                    } else if new_owner.imported {
                        new_owner.config.clone()
                    } else if previous.1.imported {
                        previous.1.config.clone()
                    } else {
                        None
                    };
                    let changed = snapshot != previous.0;
                    let disabled = new_owner.explicit
                        || (new_owner.imported
                            && new_owner.config.as_ref().map_or(true, |config| {
                                let duration = config.duration.raw_value();
                                !duration.is_finite() || duration <= 0.0
                            }));
                    if !changed && !disabled && !previous.2 {
                        previous.1 = new_owner;
                        return;
                    }
                    // Sample before cancelling the old clock or touching its endpoints.
                    let current = frozen(output.get())
                        .map(|(value, _)| value)
                        .unwrap_or_else(|_| next.clone());
                    progress.cancel_transitions();
                    let config = config.filter(|config| {
                        let duration = match config.duration {
                            Duration::Frames(v)
                            | Duration::Milliseconds(v)
                            | Duration::Seconds(v) => v.to_float(),
                        };
                        can_interpolate && duration.is_finite() && duration > 0.0
                    });
                    *endpoints.borrow_mut() = Endpoints {
                        from: current,
                        to: next,
                        curve: config.as_ref().map_or("Linear", |config| config.curve),
                    };
                    if changed && !previous.2 {
                        if let Some(config) = config {
                            progress.set(0.0);
                            progress.ease_to(1.0, config.duration, EasingCurve::Linear);
                        } else {
                            progress.set(1.0);
                        }
                    } else {
                        progress.set(1.0);
                    }
                    revision.set(revision.get().wrapping_add(1));
                    *previous = (snapshot, new_owner, false);
                },
                &dependencies,
                "settings target listener",
            )
        };
        register_effect_property(&listener);
        // Establish the baseline before the first render, even for an unobserved value.
        listener.get();
        Some(Self {
            target,
            owner,
            progress,
            _listener: listener,
        })
    }
}

/// Applies a resolved target while retaining a receiver's active settings motion.
/// Generated cartridges call this after resolving all precedence layers.
#[doc(hidden)]
pub fn bind_settings_property<T: PropertyValue + CoercionRules + ToPaxValue>(
    output: &Property<T>,
    target: Property<T>,
    name: &str,
    entries: &[RuntimeResolvedPropertyEntry],
    stack: &Rc<RuntimePropertiesStackFrame>,
    node: Option<&ExpandedNode>,
) {
    bind_settings_property_with(output, target, name, entries, stack, node, T::interpolate);
}

pub(crate) fn bind_settings_property_with<T: PropertyValue + CoercionRules + ToPaxValue>(
    output: &Property<T>,
    target: Property<T>,
    name: &str,
    entries: &[RuntimeResolvedPropertyEntry],
    stack: &Rc<RuntimePropertiesStackFrame>,
    node: Option<&ExpandedNode>,
    interpolate: impl Fn(&T, &T, f64) -> T + 'static,
) {
    let Some(node) = node else {
        output.replace_with(target);
        return;
    };
    let mut motions = node.settings_motion.borrow_mut();
    // Unconditional explicit ownership cannot be affected by an imported layer.
    let explicit = entries.last().map_or(false, |entry| {
        entry.condition.is_none()
            && (matches!(entry.source, RuntimeSettingsSource::Inline)
                || matches!(
                    entry.value,
                    ValueDefinition::Timeline(_)
                        | ValueDefinition::Transition(_)
                        | ValueDefinition::DoubleBinding(_)
                ))
    });
    if explicit {
        motions.remove(name);
        output.replace_with(target);
        return;
    }
    if let Some(motion) = motions
        .get_mut(name)
        .and_then(|motion| motion.downcast_mut::<SettingsMotion<T>>())
    {
        motion.target.replace_with(target);
        motion.owner.replace_with(owner_property(entries, stack));
        return;
    }
    let enabled = entries
        .iter()
        .any(|entry| entry.transition.as_ref().and_then(Property::get).is_some());
    if !enabled {
        output.replace_with(target);
        return;
    }
    let owner = owner_property(entries, stack);
    if let Some(motion) = SettingsMotion::new(
        output,
        target.clone(),
        owner,
        current_frame() != node.settings_birth_frame,
        Rc::new(interpolate),
    ) {
        motions.insert(name.to_owned(), Box::new(motion));
    } else {
        output.replace_with(target);
    }
}

/// Releases automatic motion before a two-way binding takes direct ownership.
#[doc(hidden)]
pub fn clear_settings_motion(node: Option<&ExpandedNode>, name: &str) {
    if let Some(node) = node {
        node.settings_motion.borrow_mut().remove(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_runtime_api::properties::{
        drain_effects, property_has_direct_outbound, register_millis, register_time,
    };

    fn clock() -> Property<u64> {
        register_time(&Property::new(0));
        let millis = Property::new(0);
        register_millis(&millis);
        millis
    }

    fn owner() -> Property<Owner> {
        Property::new(Owner {
            imported: true,
            explicit: false,
            config: Some(SettingsTransitionConfig {
                duration: Duration::Milliseconds(100.into()),
                curve: "Linear",
            }),
        })
    }

    fn settle() {
        drain_effects(100);
    }

    #[test]
    fn retarget_samples_current_clock_and_restarts_full_duration() {
        let millis = clock();
        let output = Property::new(0.0);
        let source = Property::new(0.0);
        let motion = SettingsMotion::new(
            &output,
            source.clone(),
            owner(),
            false,
            Rc::new(f64::interpolate),
        )
        .unwrap();
        source.set(10.0);
        settle();
        millis.set(40);
        // Deliberately don't pull the old output before interrupting it.
        source.set(20.0);
        settle();
        assert_eq!(output.get(), 4.0);
        millis.set(90);
        assert_eq!(output.get(), 12.0);
        millis.set(140);
        assert_eq!(output.get(), 20.0);
        millis.set(141);
        assert_eq!(output.get(), 20.0);
        assert!(!property_has_direct_outbound(
            &millis.untyped(),
            &motion.progress.untyped()
        ));
        // Source remains live after completing the first two generations.
        source.set(0.0);
        settle();
        millis.set(191);
        assert_eq!(output.get(), 10.0);
    }

    #[test]
    fn initial_value_and_equal_targets_do_not_animate_or_restart() {
        let millis = clock();
        let output = Property::new(0.0);
        let source = Property::new(10.0);
        let motion = SettingsMotion::new(
            &output,
            source.clone(),
            owner(),
            false,
            Rc::new(f64::interpolate),
        )
        .unwrap();
        assert_eq!(output.get(), 10.0);
        assert!(!property_has_direct_outbound(
            &millis.untyped(),
            &motion.progress.untyped()
        ));
        source.set(20.0);
        settle();
        millis.set(40);
        source.set(20.0);
        settle();
        millis.set(100);
        assert_eq!(output.get(), 20.0);
    }

    #[test]
    fn rebind_preserves_motion_and_instances_have_independent_clocks() {
        let millis = clock();
        let a = Property::new(0.0);
        let b = Property::new(0.0);
        let a_target = Property::new(0.0);
        let b_target = Property::new(0.0);
        let first = SettingsMotion::new(
            &a,
            a_target.clone(),
            owner(),
            false,
            Rc::new(f64::interpolate),
        )
        .unwrap();
        let _second = SettingsMotion::new(
            &b,
            b_target.clone(),
            owner(),
            false,
            Rc::new(f64::interpolate),
        )
        .unwrap();
        a_target.set(10.0);
        b_target.set(10.0);
        settle();
        millis.set(40);
        first.target.replace_with(Property::new(20.0));
        settle();
        millis.set(90);
        assert_eq!(a.get(), 12.0);
        assert_eq!(b.get(), 9.0);
    }

    #[test]
    fn disabling_snaps_and_detaches_clock() {
        let millis = clock();
        let output = Property::new(0.0);
        let target = Property::new(0.0);
        let policy = owner();
        let motion = SettingsMotion::new(
            &output,
            target.clone(),
            policy.clone(),
            false,
            Rc::new(f64::interpolate),
        )
        .unwrap();
        target.set(10.0);
        settle();
        millis.set(40);
        policy.update(|policy| policy.config = None);
        settle();
        assert_eq!(output.get(), 10.0);
        assert!(!property_has_direct_outbound(
            &millis.untyped(),
            &motion.progress.untyped()
        ));
    }

    #[test]
    fn easing_overshoot_is_not_mistaken_for_completion() {
        let millis = clock();
        let output = Property::new(0.0);
        let target = Property::new(0.0);
        let policy = owner();
        policy.update(|policy| policy.config.as_mut().unwrap().curve = "OutBack");
        let _motion = SettingsMotion::new(
            &output,
            target.clone(),
            policy,
            false,
            Rc::new(f64::interpolate),
        )
        .unwrap();
        target.set(10.0);
        settle();
        millis.set(70);
        assert!(output.get() > 10.0);
        millis.set(100);
        assert_eq!(output.get(), 10.0);
    }

    #[test]
    fn compound_endpoints_do_not_keep_live_property_handles() {
        use pax_runtime_api::{Size, Stroke};
        let millis = clock();
        let width = Property::new(Size::Pixels(10.into()));
        let source = {
            let width = width.clone();
            let dependencies = [width.untyped()];
            Property::computed(
                move || {
                    let _ = width.get();
                    Stroke {
                        width: width.clone(),
                        ..Stroke::default()
                    }
                },
                &dependencies,
            )
        };
        let output = Property::new(Stroke::default());
        let _motion = SettingsMotion::new(
            &output,
            source,
            owner(),
            false,
            Rc::new(Stroke::interpolate),
        )
        .unwrap();
        width.set(Size::Pixels(20.into()));
        settle();
        millis.set(50);
        assert_eq!(output.get().width.get().expect_pixels().to_float(), 15.0);
        width.set(Size::Pixels(40.into()));
        settle();
        assert_eq!(output.get().width.get().expect_pixels().to_float(), 15.0);
        millis.set(100);
        assert_eq!(output.get().width.get().expect_pixels().to_float(), 27.5);
    }

    #[test]
    fn zero_duration_configuration_applies_pending_target_immediately() {
        let millis = clock();
        let output = Property::new(0.0);
        let source = Property::new(0.0);
        let policy = owner();
        let motion = SettingsMotion::new(
            &output,
            source.clone(),
            policy.clone(),
            false,
            Rc::new(f64::interpolate),
        )
        .unwrap();
        source.set(10.0);
        settle();
        millis.set(40);
        policy.update(|owner| {
            owner.config.as_mut().unwrap().duration = Duration::Milliseconds(0.into())
        });
        settle();
        assert_eq!(output.get(), 10.0);
        assert!(!property_has_direct_outbound(
            &millis.untyped(),
            &motion.progress.untyped()
        ));
    }

    #[test]
    fn conditional_winner_controls_motion_not_the_last_structural_entry() {
        use pax_manifest::{ExpressionInfo, TypeId};
        use pax_runtime_api::Variable;
        use std::collections::HashMap;
        let millis = clock();
        let condition = Property::new(false);
        let base = Property::new(0.0);
        let imported = Property::new(10.0);
        let stack = RuntimePropertiesStackFrame::new(HashMap::from([(
            "enabled".into(),
            Variable::new_from_typed_property(condition.clone()),
        )]));
        let condition_info = crate::RuntimeSettingsCondition {
            positive: vec![ExpressionInfo::new(
                pax_language::parse_pax_expression("enabled").unwrap(),
            )],
            negative: vec![],
        };
        let entries = vec![RuntimeResolvedPropertyEntry {
            source: RuntimeSettingsSource::ImportedLayer {
                provider_id: crate::ExpandedNodeIdentifier(1),
                provider_type_id: TypeId::default(),
            },
            transition: Some(Property::new(owner().get().config)),
            selector: None,
            source_location: None,
            source_stack: Some(stack.clone()),
            value: ValueDefinition::Undefined,
            condition: Some(condition_info.clone()),
            axis_index: None,
        }];
        let target = crate::apply_runtime_settings_condition(
            "value",
            Some(&condition_info),
            &stack,
            base.clone(),
            imported.clone(),
        );
        let output = Property::new(0.0);
        let _motion = SettingsMotion::new(
            &output,
            target,
            owner_property(&entries, &stack),
            false,
            Rc::new(f64::interpolate),
        )
        .unwrap();
        base.set(2.0);
        imported.set(20.0);
        settle();
        assert_eq!(output.get(), 2.0); // The losing import does not animate this change.
        condition.set(true);
        settle();
        millis.set(50);
        assert_eq!(output.get(), 11.0);
        condition.set(false);
        settle();
        millis.set(100);
        assert_eq!(output.get(), 6.5); // Removal uses the outgoing import policy.
    }
}
