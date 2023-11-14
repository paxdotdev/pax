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
