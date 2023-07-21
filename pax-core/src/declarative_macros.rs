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
