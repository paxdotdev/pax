// Taken from: https://users.rust-lang.org/t/dump-all-refcell-borrows/28315/4
::cfg_if::cfg_if! { if #[cfg(debug_assertions)] {
    #[derive(
        Debug,
        Clone,
        Default,
        PartialEq, Eq,
        PartialOrd, Ord,
    )]
    pub struct RefCell<T> {
        pub ref_cell: ::core::cell::RefCell<T>,

        pub last_borrow_context: ::core::cell::Cell<&'static str>,
    }

    impl<T> RefCell<T> {
        pub fn new (value: T) -> Self
        {
            RefCell {
                ref_cell: ::core::cell::RefCell::new(value),
                last_borrow_context: ::core::cell::Cell::new(""),
            }
        }
    }

    #[macro_export]
    macro_rules! borrow {(
        $wrapper:expr
    ) => ({
        let wrapper = &$wrapper;
        if let Ok(ret) = wrapper.ref_cell.try_borrow() {
            wrapper
                .last_borrow_context
                .set(concat!(
                    "was still borrowed from ",
                    file!(), ":", line!(), ":", column!(),
                    " on expression ",
                    stringify!($wrapper),
                ));
            ret
        } else {
            panic!(
                "Error, {} {}",
                stringify!($wrapper),
                wrapper.last_borrow_context.get(),
            );
        }
    })}

    #[macro_export]
    macro_rules! borrow_mut {(
        $wrapper:expr
    ) => ({
        let wrapper = &$wrapper;
        if let Ok(ret) = $wrapper.ref_cell.try_borrow_mut() {
            $wrapper
                .last_borrow_context
                .set(concat!(
                    "was still mutably borrowed from ",
                    file!(), ":", line!(), ":", column!(),
                    " on expression ",
                    stringify!($wrapper),
                ));
            ret
        } else {
            panic!(
                "Error, {} {}",
                stringify!($wrapper),
                wrapper.last_borrow_context.get(),
            );
        }
    })}

    #[macro_export]
    macro_rules! use_RefCell {() => (
        pub use pax_runtime_api::RefCell;
    )}
} else {
    #[macro_export]
    macro_rules! borrow {(
        $ref_cell:expr
    ) => (
        $ref_cell.borrow()
    )}

    #[macro_export]
    macro_rules! borrow_mut {(
        $ref_cell:expr
    ) => (
        $ref_cell.borrow_mut()
    )}
    #[macro_export]
    macro_rules! use_RefCell {() => (
        use ::core::cell::RefCell;
    )}
}}
