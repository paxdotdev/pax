use std::collections::{HashMap, VecDeque};
use crate::manifest::{PropertyDefinition, TypeDefinition};


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
// pub fn populate_type_definition(original_type: &str, fully_qualified_constituent_types: Vec<String>, dep_to_fqd_map: &HashMap<&str, String>, sub_properties: Option<HashMap<String, PropertyDefinition>>, iterable_type: Option<Box<TypeDefinition>>) -> TypeDefinition {
//
//     let mut fully_qualified_type = original_type.to_string();
//
//     //extract dep_to_fqd_map into a Vec<String>; string-replace each looked-up value present in
//     //unexpanded_path, ensuring that each looked-up value is not preceded by a `::`
//     dep_to_fqd_map.keys().for_each(|key| {
//         fully_qualified_type.clone().match_indices(key).for_each(|i|{
//             if i.0 < 2 || {let maybe_coco : String = fully_qualified_type.chars().skip((i.0 as i64) as usize - 2).take(2).collect(); maybe_coco != "::" } {
//                 let new_value = "{PREFIX}".to_string() + &dep_to_fqd_map.get(key).unwrap();
//                 let starting_index : i64 = i.0 as i64;
//                 let end_index_exclusive = starting_index + key.len() as i64;
//                 fully_qualified_type.replace_range(starting_index as usize..end_index_exclusive as usize, &new_value);
//             }
//         });
//     });
//
//     let fully_qualified_type_pascalized = escape_identifier(fully_qualified_type.clone());
//
//     let property_definitions = ctx.all_property_definitions.get(source_id).unwrap().clone();
//
//
//     TypeDefinition {
//         original_type: original_type.to_string(),
//         fully_qualified_type,
//         fully_qualified_type_pascalized,
//         fully_qualified_constituent_types,
//         iterable_type,
//         sub_properties,
//     }
// }

