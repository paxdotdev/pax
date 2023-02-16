use std::collections::HashMap;
use futures::stream::iter;
use crate::manifest::PropertyType;


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

    let iterable_type = if fully_qualified_type.starts_with("Vec<") {
        //Statically retrieve to `Vec<T>`'s `T` through hacky-af string edits
        //This will be a problem down the line any time a Vec is qualified in place e.g. as `std::collections:Vec<T>` instead of `Vec<T>` — or if Vec is aliased at import-time.
        let mut iterable_type_id = fully_qualified_type.clone().replacen("Vec<", "", 1);
        //remove final angle bracket
        iterable_type_id.remove(iterable_type_id.len() - 1);

        //Note: recursive
        Some(Box::new(expand_fully_qualified_type_and_pascalize(&iterable_type_id, dep_to_fqd_map) ))
    } else {
        None
    };

    PropertyType {
        pascalized_fully_qualified_type,
        fully_qualified_type,
        iterable_type
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