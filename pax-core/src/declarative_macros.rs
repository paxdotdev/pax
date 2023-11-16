/// Manages unpacking an Rc<RefCell<dyn Any>>, downcasting into
/// the parameterized `target_type`, and executing a provided closure `body` in the
/// context of that unwrapped variant (including support for mutable operations),
/// then cleaning up by repacking that variant into the Rc<RefCell<>> after
/// the closure is executed.  Used at least by calculating properties in `expand_node` and
/// passing `&mut self` into event handlers (where the typed `self` is retrieved from an instance of `dyn Any`)
#[macro_export]
macro_rules! with_properties_unwrapped {
    ($rc_refcell_any_properties:expr, $target_type:ty, $body:expr) => {{
        // Clone rc to ensure lifetime alignment
        let rc = $rc_refcell_any_properties.clone();

        // Borrow the contents of the RefCell mutably.
        let mut borrowed = rc.borrow_mut();

        // Downcast the unwrapped value to the specified `target_type` (or panic)
        let mut unwrapped_value = if let Some(val) = (&mut *borrowed).downcast_mut::<$target_type>()
        {
            val
        } else {
            panic!()
        }; // Failed to downcast

        // Evaluate the passed closure and return its return value
        $body(&mut unwrapped_value)
    }};
}


/// Manages vtable updates (if necessary) for a given `dyn PropertyInstance`, with the provided expected TypesCoproduct variant.
/// Is a no-op for `PropertyLiteral`s, and mutates (by calling `.set`) `PropertyExpression` instances.
/// # Examples
/// ```text
/// handle_vtable_update!(ptc, self.height, Size);
/// ```
#[macro_export]
macro_rules! handle_vtable_update {
    ($ptc:expr, $var:ident . $field:ident, $inner_type:ty) => {{
        assert!($ptc.current_expanded_node.is_some());//Cannot update vtable without first registering an ExpandedNode on ptc
        let current_prop = &mut *$var.$field.as_mut();
        if let Some(vtable_id) = current_prop._get_vtable_id() {
            let new_value_wrapped: Box<dyn Any> = $ptc.compute_vtable_value(vtable_id);
            if let Ok(downcast_value) = new_value_wrapped.downcast::<$inner_type>() {
                current_prop.set(*downcast_value);
            } else {
                panic!()
            } //downcast failed
        }
    }};
}

/// Does same as [`handle_vtable_update`], but manages case (as a no-op) where the property is wrapped in an outer Option,
/// e.g. for CommonProperties.
/// # Examples
/// ```text
/// // In this example `scale_x` is `Option`al (`Option<Rc<RefCell<dyn PropertyInstance<Size>>>>`)
/// handle_vtable_update_optional!(ptc, self.scale_x, Size);
/// ```
#[macro_export]
macro_rules! handle_vtable_update_optional {
    ($ptc:expr, $var:ident . $field:ident, $inner_type:ty) => {{
        assert!($ptc.current_expanded_node.is_some());//Cannot update vtable without first registering an ExpandedNode on ptc
        if let Some(_) = $var.$field {
            let current_prop = &mut *$var.$field.as_mut().unwrap();

            if let Some(vtable_id) = current_prop._get_vtable_id() {
                let new_value_wrapped: Box<dyn Any> = $ptc.compute_vtable_value(vtable_id);
                if let Ok(downcast_value) = new_value_wrapped.downcast::<$inner_type>() {
                    current_prop.set(*downcast_value);
                } else {
                    panic!()
                } //downcast failed
            }
        }
    }};
}