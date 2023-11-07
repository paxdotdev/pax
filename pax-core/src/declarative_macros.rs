/// Extracts the target value from an enum using raw memory access.
///
/// Parameters:
/// - `$source_enum`: The enum instance to extract the target value from.
/// - `$enum_type`: The type of the enum.
/// - `$target_type`: The type of the target value to extract.
#[macro_export]
macro_rules! unsafe_unwrap {
    ($source_enum:expr, $enum_type:ty, $target_type:ty) => {{
        fn unwrap_impl<T, U: Default>(source_enum: T) -> U {
            let size_of_enum = std::mem::size_of::<T>();
            let size_of_target = std::mem::size_of::<U>();
            let align_of_enum = std::mem::align_of::<T>();

            assert!(size_of_target < size_of_enum, "The size_of target_type must be less than the size_of enum_type.");

            let mut boxed_enum = Box::new(source_enum);
            let mut default_value = U::default();

            let target = unsafe {
                let enum_ptr = Box::into_raw(boxed_enum);
                let target_ptr = (enum_ptr as *mut u8).add(align_of_enum) as *mut U;

                std::mem::swap(&mut *target_ptr, &mut default_value);

                // We no longer need the boxed enum, so it can be safely dropped.
                // Note that because the value inside the enum variant was replaced with a default value,
                // dropping this box does not drop the original value.
                drop(Box::from_raw(enum_ptr));

                default_value
            };
            target
        }
        unwrap_impl::<$enum_type, $target_type>($source_enum)
    }};
}

#[macro_export]
macro_rules! unsafe_wrap {
    ($value:expr, $enum_type:ty, $target_type:ty) => {{
        fn wrap_impl<T: Default, U: Default>(value: &U) -> T {
            let size_of_enum = std::mem::size_of::<T>();
            let size_of_value = std::mem::size_of::<U>();
            let align_of_enum = std::mem::align_of::<T>();

            assert!(size_of_value < size_of_enum, "The size_of target_type must be less than the size_of enum_type.");

            let mut boxed_enum = Box::new(T::default()); // Assuming your enum has a Default impl.

            unsafe {
                let enum_ptr = Box::into_raw(boxed_enum);
                let value_ptr = value as *const U;  // Directly take the pointer from the reference
                let target_ptr = (enum_ptr as *mut u8).add(align_of_enum) as *mut U;

                std::ptr::copy_nonoverlapping(value_ptr, target_ptr, 1); // Use copy_nonoverlapping since source and destination won't overlap

                // Transfer ownership of the enum back to Rust for proper handling
                *Box::from_raw(enum_ptr)
            }
        }
        wrap_impl::<$enum_type, $target_type>(&$value)
    }};
}

/// Manages unpacking an Rc<RefCell<PropertiesCoproduct>>, [`unsafe_unwrap!`]ping into
/// the parameterized variant/type, and executing a provided closure in the
/// context of that unwrapped variant (including support for mutable operations),
/// then cleaning up by repacking that variant into the Rc<RefCell<>> after
/// the closure is executed.  Used at least by calculating properties in `handle_compute_properties` and
/// passing `&mut self` into event handlers (where the `self` is one of these wrapped instances of PropertiesCoproduct.)
///
/// # Examples
///
/// ```text
/// let wrapped : Rc<RefCell<PropertiesCoproduct>> = foo();
/// with_properties_unsafe!(&wrapped, PropertiesCoproduct, Color, |color| {
///     // color is of type &mut Color
///     // Perform operations on `color` here.
///     // This macro will handle repacking `color` into the Rc<RefCell<PropertiesCoproduct>>
///     // after the block is evaluated.
/// });
/// ```
#[macro_export]
macro_rules! with_properties_unsafe {
    ($rc_refcell:expr, $enum_type:ty, $target_type:ty, $body:expr) => {{
        // Clone the `Rc` to ensure that we have a temporary ownership of the `RefCell`.
        let rc = $rc_refcell.clone();
        // Borrow the `RefCell` mutably and take the value, leaving `Default::default()` in its place.
        let mut value = std::mem::replace(&mut *rc.borrow_mut(), Default::default());

        // Use the unsafe_unwrap! macro to get the unwrapped value of the specific type.
        let mut unwrapped_value: $target_type = unsafe_unwrap!(value, $enum_type, $target_type);

        // This ensures that the lifetime of the reference passed to the closure does not outlive the temporary value.
        {
            let closure: Box<dyn FnOnce(&mut $target_type)> = Box::new($body);
            closure(&mut unwrapped_value);
        }

        // Wrap the enum variant back into the enum
        let rewrapped_value = unsafe_wrap!(unwrapped_value, $enum_type, $target_type);

        // Replace the potentially modified value back into the `RefCell`.
        let mut r = rc.borrow_mut();
        *r = rewrapped_value;
    }};
}

