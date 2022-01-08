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
    Empty,
}