
//A manifest for a Properties object is a list of
//key-type pairs (like `("name_label", "String")`), describing each Property in the object
//in a runtime-accessible way.  This is used e.g. for parsing and for design-tooling.

pub trait Manifestable {
    fn get_manifest() -> &'static Vec<(&'static str, &'static str)>;
}


// Describes how to "patch" a properties object with sparse masks of
// keys/values.
//
// That is:  each property is wrapped in an Option<> then
// initialized to None.
//
// By populating only the relevant fields with a Some(value),
// this `Patch` object can be used to describe partial properties, e.g. when parsing
// `{a: 1}`-style object literals, or when setting values from a design tool.
pub trait Patchable<P> {
    fn patch(&mut self, patch: P);
}


