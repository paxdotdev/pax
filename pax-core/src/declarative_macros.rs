/// Extracts the target value from an enum using raw memory access.
///
/// Parameters:
/// - `$source_enum`: The enum instance to extract the target value from.
/// - `$enum_type`: The type of the enum.
/// - `$target_type`: The type of the target value to extract.
#[macro_export]
macro_rules! unsafe_unwrap {
    ($source_enum:expr, $enum_type:ty, $target_type:ty) => {{
        fn unwrap_impl<T, U>(source_enum: T) -> (U, *mut T) {
            let size_of_enum = std::mem::size_of::<T>();
            let size_of_target = std::mem::size_of::<U>();
            let align_of_enum = std::mem::align_of::<T>();

            assert!(size_of_target <= size_of_enum, "The size_of target_type must be less than or equal to the size_of enum_type.");

            let boxed_enum = Box::new(source_enum);
            let enum_ptr = Box::into_raw(boxed_enum);
            let ptr_to_text = unsafe { ((enum_ptr as *const u8).add(align_of_enum) as *const U) };
            let target_copy = unsafe { ptr_to_text.read() };

            (target_copy, enum_ptr)
        }
        unwrap_impl::<$enum_type, $target_type>($source_enum)
    }};
}

#[macro_export]
macro_rules! unsafe_cleanup {
    ($raw_ptr:expr, $type:ty) => {{
        unsafe {
            let _boxed = Box::from_raw($raw_ptr as *mut $type);
            // The Box is immediately dropped here, deallocating the memory.
        }
    }};
}
