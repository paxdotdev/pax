
/*

TODO:
    [ ] CLI + args parsing
        [ ] Pass path to HTML file or multiple files, or to a .json config
    [ ] Template parser
    [ ] Properties parser
    [ ] Expression parser

*/



/*
COMPILATION STAGES

0. Process template
    - build render tree by parsing template file
        - unroll @{} into a vanilla tree (e.g. `<repeat>` instead of `foreach`)
        - defer inlined properties & expressions to `process properties` step
    - semantize: map node keys to known rendernode types
    - fails upon malformed tree or unknown node types
1. Process properties
    - link properties to nodes of render tree
    - first parse "stylesheet" properties
    - semantize: map selectors to known template nodes+types, then property keys/values to known node properies + FromString=>values
    - then override with inlined properties from template
    - fails upon type mismatches, empty-set selectors, heterogenous multi-element selectors
2. Process expressions
    - parse & lambda-ize expressions, applying a la properties above
    - return primitive types
    - fails upon return type mismatch, malformed expression
 */

fn main() {
    println!("Hello, world!");
}
