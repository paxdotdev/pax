
/*

TODO:
    [ ] CLI + args parsing
        [ ] Pass path to HTML file or multiple files, or to a .json config
    [ ] DOM parser
        [ ] Headless browser load HTML file
    [ ] Double-brace (`{{}}`) templating
        [ ] Parsing mechanism for `{{}}`, meeting half-way on syntax if needed
        [ ] Mechanism for mapping rust fragments to PropertyValues
            [ ] Sanity-check home-rolled expression syntax, PEG
    [ ] wasm compiler-chassis
        [ ] Embed alongside loaded HTML, grab DOM structure and attributes

*/



/*
COMPILATION STAGES

0. Process double-braces
    - parse file for {{}} pairs, swapping content with a known string attribute,
      or otherwise referencable property at runtime in the hydrated DOM
    - either: compile the expression language via PEG to rust and then compile onto target
          or: copy-paste the provided code snippets into a generated rust wrapper file, compile
       * Keep in mind * whatever approach is viable for "code-behind" (`<script type="application/x-rust">`)
1. Load compiled HTML into headless browser, along with wasm compiler-chassis
    - Pull .rs code out of `<script>` declarations, plop into templates, compile with rustc/cargo
    - Use web-sys to parse the hydrated DOM and map into carbon """bytecode"""

 */

fn main() {
    println!("Hello, world!");
}   
