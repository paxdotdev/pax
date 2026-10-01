use super::*;

impl<T: PropertyValue> Interpolatable for LocalProperty<T> {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        let cp_self = self.clone();
        let cp_other = other.clone();
        LocalProperty::computed(
            move || cp_self.get().interpolate(&cp_other.get(), t),
            &[self.untyped(), other.untyped()],
        )
    }
}
/// An owner-thread value or computation in Pax's reactive graph.
///
/// Unlike [`Property`], this handle may contain local values and evaluator
/// captures. It cannot cross threads, even when `T` itself is transferable.
///
/// ```compile_fail
/// use pax_runtime_api::LocalProperty;
/// let value = LocalProperty::new(1_u32);
/// std::thread::spawn(move || value.get());
/// ```
#[derive(Clone)]
pub struct LocalProperty<T> {
    pub(super) untyped: UntypedProperty,
    _phantom: PhantomData<T>,
}

impl<T: PropertyValue> PropertyBinding<T> for LocalProperty<T> {
    fn from_value(value: T) -> Self {
        Self::new(value)
    }
    fn local(&self) -> Self {
        self.clone()
    }
    fn from_local(source: Self) -> Self {
        source
    }
    fn value_for_conversion(&self) -> T {
        self.get()
    }
}

impl<T: PropertyValue + std::fmt::Debug> std::fmt::Debug for LocalProperty<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LocalProperty ({:?})", self.get())
    }
}

impl<T: PropertyValue> LocalProperty<T> {
    /// Clones this owner-thread view without changing its property identity.
    pub fn local(&self) -> Self {
        self.clone()
    }

    /// Creates a literal property with an initial value.
    pub fn new(val: T) -> Self {
        Self::new_optional_name(val, None)
    }

    // Rewraps an untyped property with a typed view.
    pub fn new_from_untyped(untyped: UntypedProperty) -> Self {
        Self {
            untyped,
            _phantom: PhantomData {},
        }
    }

    /// Creates a computed property from an evaluator and dependency list.
    pub fn computed(evaluator: impl Fn() -> T + 'static, dependents: &[UntypedProperty]) -> Self {
        Self::computed_with_config(evaluator, dependents, None, None)
    }

    /// Creates a computed property with a propagation cutoff.
    ///
    /// The predicate receives the last accepted value and the newly evaluated
    /// candidate. Returning `true` discards the candidate and stops outbound
    /// invalidation at this property; returning `false` accepts and propagates
    /// it. The first evaluation is always accepted.
    pub fn computed_with_cutoff(
        evaluator: impl Fn() -> T + 'static,
        dependents: &[UntypedProperty],
        cutoff: impl Fn(&T, &T) -> bool + 'static,
    ) -> Self {
        Self::computed_with_config(evaluator, dependents, None, Some(Rc::new(cutoff)))
    }

    /// Creates a named literal property, useful for diagnostics.
    pub fn new_with_name(val: T, name: &str) -> Self {
        Self::new_optional_name(val, Some(name))
    }

    /// Creates a named computed property, useful for diagnostics.
    pub fn computed_with_name(
        evaluator: impl Fn() -> T + 'static,
        dependents: &[UntypedProperty],
        name: &str,
    ) -> Self {
        Self::computed_with_config(evaluator, dependents, Some(name), None)
    }

    /// Creates a named cutoff computed property, useful for diagnostics.
    pub fn computed_with_cutoff_and_name(
        evaluator: impl Fn() -> T + 'static,
        dependents: &[UntypedProperty],
        cutoff: impl Fn(&T, &T) -> bool + 'static,
        name: &str,
    ) -> Self {
        Self::computed_with_config(evaluator, dependents, Some(name), Some(Rc::new(cutoff)))
    }

    fn new_optional_name(val: T, name: Option<&str>) -> Self {
        Self {
            untyped: UntypedProperty::new(val, Vec::with_capacity(0), PropertyType::Literal, name),
            _phantom: PhantomData {},
        }
    }

    fn computed_with_config(
        evaluator: impl Fn() -> T + 'static,
        dependents: &[UntypedProperty],
        name: Option<&str>,
        cutoff: Option<Rc<dyn Fn(&T, &T) -> bool>>,
    ) -> Self {
        let inbound: Vec<_> = dependents.iter().map(|v| v.get_id()).collect();
        let start_val = T::default();
        let evaluator = Rc::new(super::shared_graph::owner_evaluator(evaluator));
        Self {
            untyped: UntypedProperty::new(
                start_val,
                inbound,
                PropertyType::Computed {
                    evaluator,
                    cutoff: cutoff.map(|predicate| properties_table::Cutoff {
                        predicate,
                        initialized: false,
                    }),
                },
                name,
            ),
            _phantom: PhantomData {},
        }
    }

    /// Immediately starts an ease transition from the current value to end_val, over a duration, following curve.
    ///
    /// Numeric arguments preserve the historical frame-based behavior. Use
    /// `Duration::Milliseconds`, `Duration::Seconds`, or `Duration::Frames` to
    /// select an explicit unit.
    pub fn ease_to<D: Into<Duration>>(&self, end_val: T, duration: D, curve: EasingCurve) {
        self.ease_to_value(end_val, duration.into(), curve, true);
    }

    /// Enqueues an ease transition from the current value to end_val, over a duration, following
    /// curve, which will start after all currently enqueued transitions finish.
    pub fn ease_to_later<D: Into<Duration>>(&self, end_val: T, duration: D, curve: EasingCurve) {
        self.ease_to_value(end_val, duration.into(), curve, false);
    }

    /// Shared logic for easing operations
    fn ease_to_value(&self, end_val: T, duration: Duration, curve: EasingCurve, overwrite: bool) {
        PROPERTY_TABLE.with(|t| {
            t.transition(
                self.untyped.id,
                TransitionQueueEntry {
                    duration,
                    curve,
                    ending_value: end_val,
                },
                overwrite,
            )
        })
    }

    /// Stops the active transition and clears every queued transition segment.
    ///
    /// The property is left at its current eased value. A subsequent [`LocalProperty::set`]
    /// can therefore take immediate ownership without the cancelled transition
    /// overwriting it on the next runtime tick.
    pub fn cancel_transitions(&self) {
        PROPERTY_TABLE.with(|t| t.cancel_transitions::<T>(self.untyped.id));
    }

    /// Gets the currently stored value. Might be computationally
    /// expensive in a large reactivity network since this triggers
    /// re-evaluation of dirty property chains
    pub fn get(&self) -> T {
        PROPERTY_TABLE.with(|t| t.get_value(self.untyped.id))
    }

    /// Sets this properties value and sets the dirty bit recursively of all of
    /// its dependencies if not already set
    pub fn set(&self, val: T) {
        PROPERTY_TABLE.with(|t| t.set_value(self.untyped.id, val));
    }

    /// Marks this property for re-evaluation without replacing its value or evaluator.
    ///
    /// This is useful for computed properties whose evaluator observes runtime state
    /// outside the reactive dependency graph.
    #[doc(hidden)]
    pub fn invalidate(&self) {
        PROPERTY_TABLE.with(|t| t.invalidate(self.untyped.id));
    }

    /// Sets the value only when it differs from the current one.
    ///
    /// Returns `true` when the write changed the property and dirtied dependents.
    pub fn set_if_neq(&self, val: T) -> bool
    where
        T: PartialEq,
    {
        let should_set =
            PROPERTY_TABLE.with(|t| t.read_value(self.untyped.id, |current: &T| current != &val));
        if should_set {
            self.set(val);
        }
        should_set
    }

    /// Get access to a mutable reference to the inner value T.
    /// Will trigger updates for dependents of this property, regardless
    /// of if the value actually changed
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        // This is a temporary impl of the update method.
        // (very bad perf comparatively, but very safe).
        let mut val = self.get();
        f(&mut val);
        self.set(val);
    }

    /// Reads the inner value by reference.
    ///
    /// Panics if this property is already borrowed, which can happen if `read`
    /// is called inside a read of the same property.
    pub fn read<V>(&self, f: impl FnOnce(&T) -> V) -> V {
        PROPERTY_TABLE.with(|t| t.read_value(self.untyped.id, f))
    }

    /// Replaces this property's evaluator, dependencies, and value with `target`, while keeping dependents.
    ///
    /// This can introduce circular dependencies if used carelessly. It is
    /// intended for changing a property from literal to computed (or vice
    /// versa) without severing existing outbound links.
    pub fn replace_with(&self, target: LocalProperty<T>) {
        PROPERTY_TABLE.with(|t| {
            // we know self contains T, and that target contains T, so this should never panic
            t.replace_property_keep_outbound_connections::<T>(self.untyped.id, target.untyped.id)
        })
    }

    /// Casts this property to its untyped version.
    pub fn untyped(&self) -> UntypedProperty {
        self.untyped.clone()
    }
}

impl<T: PropertyValue> Default for LocalProperty<T> {
    fn default() -> Self {
        LocalProperty::new(T::default())
    }
}

// Serialization and deserialization fully disconnects properties,
// and only loads them back in as literal values.
impl<'de, T: PropertyValue + Deserialize<'de>> Deserialize<'de> for LocalProperty<T> {
    fn deserialize<D>(deserializer: D) -> Result<LocalProperty<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;
        Ok(LocalProperty::new(value))
    }
}

impl<T: PropertyValue + Serialize> Serialize for LocalProperty<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // TODO check if literal or computed, error on computed?
        self.get().serialize(serializer)
    }
}
