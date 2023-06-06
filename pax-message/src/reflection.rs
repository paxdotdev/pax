
/// Reflectable must not depend downstream on pax-compiler, thus it has been hoisted
/// into this leaf node package (PaxMessage)
pub trait Reflectable {
    //Note: this default implementation is probably not the right approach, but it works hackily
    //      alongside e.g. `impl Stringable for i64{} pub use i64`.  A better alternative may be to `#[derive(Manifestable)]` (or
    //      derive as part of `pax`, `pax_app`, and `pax_type` macros)
    fn get_fully_qualified_path(atomic_self_type: &str) -> String; /*{
        let fully_qualified_path = module_path!().to_owned() + "::" + atomic_self_type;
        fully_qualified_path
    }*/

    ///
    fn get_type_id() -> String;

}

impl Reflectable for usize {
    fn get_fully_qualified_path(_: &str) -> String {
        "usize".to_string()
    }

    fn get_type_id() -> String {
        "usize".to_string()
    }
}

impl Reflectable for f64 {
    fn get_fully_qualified_path(_: &str) -> String {
        "f64".to_string()
    }
    fn get_type_id() -> String {
        "f64".to_string()
    }
}

impl Reflectable for bool {
    fn get_fully_qualified_path(_: &str) -> String {
        "bool".to_string()
    }

    fn get_type_id() -> String {
        "bool".to_string()
    }
}

impl Reflectable for i64 {
    fn get_fully_qualified_path(_: &str) -> String {
        "i64".to_string()
    }

    fn get_type_id() -> String {
        "i64".to_string()
    }
}

impl Reflectable for std::string::String {
    fn get_fully_qualified_path(_: &str) -> String {
        "std::string::String".to_string()
    }

    fn get_type_id() -> String {
        "std::string::String".to_string()
    }
}

impl<T> Reflectable for std::rc::Rc<T> {
    fn get_fully_qualified_path(_: &str) -> String {
        "std::rc::Rc".to_string()
    }

    fn get_type_id() -> String {
        "std::rc::Rc".to_string()
    }
}

impl<T> Reflectable for std::vec::Vec<T> {
    fn get_fully_qualified_path(_: &str) -> String {
        "std::vec::Vec".to_string()
    }

    fn get_type_id() -> String {
        "std::vec::Vec".to_string()
    }
}
