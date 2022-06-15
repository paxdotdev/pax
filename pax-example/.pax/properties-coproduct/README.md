# pax-properties-coproduct

This crate acts as a `blank` - a dummy dependency that allows projects to bootstrap their initial compilation, later
replacing this blank with an application-specific `PropertiesCoproduct`.  

In other words:  every project needs a `PropertiesCoproduct` in order to compile, but that `PropertiesCoproduct`
cannot be known until the project is initially compiled.  This `blank` allows the first compilation to occur.

The Cargo `patch` mechanism is how this blank is intended to be substituted for an application-specific
`PropertiesCoproduct`.  The `ExpressionTable` is expected to be patched in by the same mechanism.