
//A manifest for a Properties object is a list of
//key-type pairs (like `("name_label", "String")`), describing each Property in the object
//in a runtime-accessible way.  This is used e.g. for parsing and for design-tooling.

pub trait Manifestable {
    fn get_type_identifier() -> &'static str;
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







/******
The designtime tracks the following:
    - Component definitions, esp. a main component
        - a unique string `ID` (name) for the component, which becomes its fs location
          and the way it's imported/referenced.  Forward slashes may be supportable for
          nesting into directories, but we need to figure out how to resolve ambiguity
          (e.g. `Main` and `some-directory/Main`)

          Perhaps `foo::Bar`, namespace syntax?

          this needs to play nicely with Rust's import system, which will be the backbone
          for the actual import functionality. we should spot-check compatibility with ES/TS
          imports for a future web runtime.

        - a `Template`  for that component, describing a tree of its contained elements
        - `Properties` for that component's contained elements
            - a semantized object (properties coproduct) describing all properties,
              with pointers to the Expression LUT as needed
        - `Actions` for that component (or just a string representing the contents of the code-behind file)
        - An `Expression` table, really just a hashmap of `unique-id` => `function` (or just a Vec<fn>)
            - Double-check:  does this table belong in the runtime or designtime?  Is it needed
              in stand-alone runtime?

How do we compile our actions and code-behind files?
    - Generate into a temp directory?
    - pipe the content into rustc?
    - perhaps these don't get fired by the designtime (akin to Flash/HA's edit vs. preview modes) —
        that is, don't deal with actions at all in the designtime until compiling to RIL
    - perhaps the whole project is recompiled, and the designtime re-attaches itself

How does the designtime bolt onto an engine instance (+ chassis) to enable it to
"take over" the render tree definition?

Walk through the

 */

//
// pub fn operate_on_rp(mut rp: RectangleProperties) {
//
//
//     let new_patch = RectanglePropertiesPatch {
//         transform: None,
//         fill: None,
//         size: None,
//         stroke: None,
//     };
//     rp.patch(new_patch);
//
//
//     let brand_new_patch = RectanglePropertiesPatch::default();
//
//
//     // let patch_from_str = "".
// }


// Convert something like:
// {size: [40.0px, 40.0px]}
// into RectanglePropertiesPatch {}


// init default/empty Patch
// For each K,V pair in string:
// `match` K; apply V to the appropriate `patch.member`.
//    Recurse as needed, for non-primitive `member` types

//To do the above, we need a tree of types/members
//(This might also be useful for addressing & dependency management.)

//For example, RectangleProperties has members `size`, `transform`, `stroke`, and `fill`
//  Each of those members except for `size` has sub-members — `size` is a list.

//Before we get our K/V pairs, we expect the expression to be simplified/evaluated, e.g.
//  { color: {r: 100 + 1, g: (num_clicks * 10) % 360, b: 100, a: 100}}

//The above is not true!  We do NOT want the expression evaluted in the designtime context; instead we want to track the raw
//string value.  Only upon serialization to RIL do we need to evaluate the expressions further.

//Is there nuance here re: recursive property definitions?  How much do we need to evaluate them
//in the designtime?

//In the design tool we'll want to see an as-live-as-possible preview; this requires some intelligence (or, simply default values?)
//Three approaches:
// 1. just show some default value for a property (e.g. `fill`) when the value is a runtime-dynamic expression
// 2. progressive compilation: when a runtime-dynamic expression is updated, recompile and partially/dynamically load it
// 3. dynamic evaluation: (PROBABLY NOT THIS!) derive a separate runtime vs. RIL for live evaluation of expressions

// Perhaps the best path forward is a hybrid of 1 & 2 -- that is, start with #1 as a naive + simple approach.  Implement
// #2 as a future feature, when further resources are available.






/*
We need a patchable properties object

Does `transform` and `size` go into properties or are they special?
What about `id`?

By putting as much as possible into the properties object (e.g. including transform and size),
we make it easier to DRY the `compute_in_place` logic, and to enable fine-grained control of
which elements expose size/transform vs. those for which those properties are irrelevant

What will it look like to set size?

```
//expose a trait method
my_dyn_rendernode.set_size(some_size2d)
//internally, wraps this value into a property coproduct patch, applies it
```


 */



//
// pub enum NodeKind {
//     Rectangle,
//     Spread,
//     Group,
// }
//
// pub struct Node<'a> {
//     kind:  NodeKind,
//     children: Vec<&'a Node<'a>>,
// }
//
// // pub struct Component<'a> {
// //     template: &'a Node<'a>,
// //     properties: PropertiesCoproduct,
// // }
//
// pub struct Vtable {
//     functions: Vec<fn()>
// }
//
//
//
