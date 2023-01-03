use std::collections::HashMap;
use crate::manifest::PropertyType;


pub trait PathQualifiable {
    //Note: this default implementation is probably not the right approach, but it works hackily
    //      alongside e.g. `impl Stringable for i64{} pub use i64`.  A better alternative may be to `#[derive(Manifestable)]` (or
    //      derive as part of `pax`, `pax_app`, and `pax_type` macros)
    fn get_fully_qualified_path(atomic_self_type: &str) -> String {
        let fully_qualified_path = module_path!().to_owned() + "::" + atomic_self_type;
        fully_qualified_path
    }
}

//
// pub trait IterableQualifiable {
//     fn hello() {}
// }
//
// impl<T> IterableQualifiable for Iter<T> where T: PathQualifiable {
//     fn hello() {
//         T::get_fully_qualified_path("hello");
//     }
// }

impl PathQualifiable for usize {
    fn get_fully_qualified_path(atomic_self_type: &str) -> String {
        "usize".to_string()
    }
}

impl PathQualifiable for f64 {
    fn get_fully_qualified_path(atomic_self_type: &str) -> String {
        "f64".to_string()
    }
}

impl PathQualifiable for i64 {
    fn get_fully_qualified_path(atomic_self_type: &str) -> String {
        "i64".to_string()
    }
}

impl PathQualifiable for std::string::String {
    fn get_fully_qualified_path(atomic_self_type: &str) -> String {
        "std::string::String".to_string()
    }
}

impl<T> PathQualifiable for std::rc::Rc<T> {
    fn get_fully_qualified_path(atomic_self_type: &str) -> String {
        "std::rc::Rc".to_string()
    }
}

impl<T> PathQualifiable for std::vec::Vec<T> {
    fn get_fully_qualified_path(atomic_self_type: &str) -> String {
        "std::vec::Vec".to_string()
    }
}

static PRELUDE_TYPES: [&'static str; 5] = [
    "std::rc::Rc",
    "std::vec::Vec",
    "usize",
    "i64",
    "u64",
];

/// When we have a fragment of a prelude type like `Rc`, this function maps
/// that fragment to the fully qualified path `std::rc::Rc`, as a String
pub fn get_fully_qualified_prelude_type(identifier: &str) -> String {
    (PRELUDE_TYPES.iter().find(|pt|{
        pt.ends_with(identifier)
    }).expect("`get_fully_resolved_prelude_type` called on a non-prelude type")).to_string()
}

pub fn is_prelude_type(identifier: &str) -> bool {
    for x in PRELUDE_TYPES.iter() {
        if x.ends_with(identifier) {
            return true;
        }
    }
    false
}

//Returns a fully expanded path and a pascalized version of that fully expanded path — for example:
// Vec<Rc<StackerCell>> becomes
// std::collections::Vec<std::rc::Rc<pax_example::pax_reexports::pax_std::types::StackerCell>>
pub fn expand_fully_qualified_type_and_pascalize(unexpanded_path: &str, dep_to_fqd_map: &HashMap<&str, String>) -> PropertyType {

    let mut fully_qualified_type = unexpanded_path.to_string();

    //extract dep_to_fqd_map into a Vec<String>; string-replace each looked-up value present in
    //unexpanded_path, ensuring that each looked-up value is not preceded by a `::`
    dep_to_fqd_map.keys().for_each(|key| {
        fully_qualified_type.clone().match_indices(key).for_each(|i|{
            if i.0 < 2 || {let maybe_coco : String = fully_qualified_type.chars().skip((i.0 as i64) as usize - 2).take(2).collect(); maybe_coco != "::" } {
                let new_value = "{PREFIX}".to_string() + &dep_to_fqd_map.get(key).unwrap();
                let length_difference: i64 = new_value.len() as i64 - key.len() as i64;
                let starting_index : i64 = i.0 as i64;
                let end_index_exclusive = starting_index + key.len() as i64;
                fully_qualified_type.replace_range(starting_index as usize..end_index_exclusive as usize, &new_value);
            }
        });
    });

    let pascalized_fully_qualified_type = escape_identifier(fully_qualified_type.clone());

    PropertyType {
        pascalized_fully_qualified_type,
        fully_qualified_type,
    }
}

pub fn escape_identifier(input: String) -> String {
    input
        .replace("(","LPAR")
        .replace("::","COCO")
        .replace(")","RPAR")
        .replace("<","LABR")
        .replace(">","RABR")
        .replace(",","COMM")
        .replace(".","PERI")
        .replace("[","LSQB")
        .replace("]","RSQB")
}