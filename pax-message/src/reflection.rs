
/// PathQualifiable must not depend downstream on pax-compiler, thus it has been hoisted
/// into this leaf node package (PaxMessage)

pub trait PathQualifiable {
    //Note: this default implementation is probably not the right approach, but it works hackily
    //      alongside e.g. `impl Stringable for i64{} pub use i64`.  A better alternative may be to `#[derive(Manifestable)]` (or
    //      derive as part of `pax`, `pax_app`, and `pax_type` macros)
    fn get_fully_qualified_path(atomic_self_type: &str) -> String {
        let fully_qualified_path = module_path!().to_owned() + "::" + atomic_self_type;
        fully_qualified_path
    }
}

impl PathQualifiable for usize {
    fn get_fully_qualified_path(_: &str) -> String {
        "usize".to_string()
    }
}

impl PathQualifiable for f64 {
    fn get_fully_qualified_path(_: &str) -> String {
        "f64".to_string()
    }
}

impl PathQualifiable for bool {
    fn get_fully_qualified_path(_: &str) -> String {
        "bool".to_string()
    }
}

impl PathQualifiable for i64 {
    fn get_fully_qualified_path(_: &str) -> String {
        "i64".to_string()
    }
}

impl PathQualifiable for std::string::String {
    fn get_fully_qualified_path(_: &str) -> String {
        "std::string::String".to_string()
    }
}

impl<T> PathQualifiable for std::rc::Rc<T> {
    fn get_fully_qualified_path(_: &str) -> String {
        "std::rc::Rc".to_string()
    }
}

impl<T> PathQualifiable for std::vec::Vec<T> {
    fn get_fully_qualified_path(_: &str) -> String {
        "std::vec::Vec".to_string()
    }
}
