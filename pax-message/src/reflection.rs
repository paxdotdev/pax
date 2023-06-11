
/// Reflectable must not depend downstream on pax-compiler, thus it has been hoisted
/// into this leaf node package (PaxMessage)
pub trait Reflectable {
    fn get_type_id(pascal_identifier: &str) -> String;

}

impl Reflectable for usize {
    fn get_type_id(_: &str) -> String {
        "usize".to_string()
    }
}

impl Reflectable for f64 {
    fn get_type_id(_: &str) -> String {
        "f64".to_string()
    }
}

impl Reflectable for bool {
    fn get_type_id(_: &str) -> String {
        "bool".to_string()
    }
}

impl Reflectable for i64 {
    fn get_type_id(_: &str) -> String {
        "i64".to_string()
    }
}

impl Reflectable for std::string::String {
    fn get_type_id(_: &str) -> String {
        "std::string::String".to_string()
    }
}

impl<T> Reflectable for std::rc::Rc<T> {
    fn get_type_id(_: &str) -> String {
        "std::rc::Rc".to_string()
    }
}

impl<T> Reflectable for std::vec::Vec<T> {
    fn get_type_id(_: &str) -> String {
        "std::vec::Vec".to_string()
    }
}
