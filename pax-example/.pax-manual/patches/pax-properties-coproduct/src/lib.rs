use pax_example::pax_types::pax_std::primitives::rectangle::RectangleInstance;

/// ∐
/// This data structure represents all of the Component Properties that
/// exist in an application.
///
/// This enum/coproduct structure solves the problem of knowing
/// the amount of memory to allocate for `PropertiesCoproduct`s on stack frames.
/// Because our components are polymorphic (i.e. each component can have
/// a different 'shape' of Properties,) and because stack frames are stored
/// in a central data structure (the runtime stack,) we need a means of
/// storing them together.  Generics + traits don't work because we
/// need concrete access to struct fields, vs. traits which give us methods only.
///
/// Keep in mind that each PropertiesCoproduct type will have the memory footprint
/// of the LARGEST type associated.  Even an instance of `Empty` will have the memory footprint
/// of `TheMostBloatedTypeEver`, so be judicious about what gets stored in PropertiesCoproduct
/// structs (e.g. be wary of binary assets like images/multimedia!)
///
pub enum PropertiesCoproduct {
    //pascal_identifier + "(" + cartridge project name + "::exports::" + module_path + "::" + pascal_identifier}
    Rectangle(pax_example::pax_types::pax_std::primitives::rectangle::RectangleInstance),
    Group(pax_example::pax_types::pax_std::primitives::group::Group),
    Root(pax_example::Root),
}

pub enum PatchCoproduct {

    // Rectangle(pax_example::exports::pax_std::primitives::rectangle::Rectangle),
    // Group(pax_example::exports::pax_std::primitives::group::Group),
    RootPatch(pax_example::RootPatch),
}
