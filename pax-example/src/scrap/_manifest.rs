//TODO: code-gen this file via macros — refer to this trick from Pest, which
// enables one-line `#[grammar = "pax-core.pest"]` linking:
/*
From: https://github.com/pest-parser/pest/blob/master/generator/src/generator.rs
```
// Needed because Cargo doesn't watch for changes in grammars.
fn generate_include(name: &Ident, path: &str) -> TokenStream {
    let const_name = Ident::new(&format!("_PEST_GRAMMAR_{}", name), Span::call_site());
    // Need to make this relative to the current directory since the path to the file
    // is derived from the CARGO_MANIFEST_DIR environment variable
    let mut current_dir = std::env::current_dir().expect("Unable to get current directory");
    current_dir.push(path);
    let relative_path = current_dir.to_str().expect("path contains invalid unicode");
    quote! {
        #[allow(non_upper_case_globals)]
        const #const_name: &'static str = include_str!(#relative_path);
    }
}
```
 */

//TODO: expose Properties manifests

//TODO: generate Properties Coproduct — does this happen here, in main, or in yet a different file?
//      Does userland code interact explicitly with PropertiesCoproducts?  (Answer: yes, in Actions)
//      As a result of the above, the PropertiesCoproduct should probably live in a "userland" file —
//      perhaps Main, where it can be expanded via a macro that inspects this manifest file.
//     Until that's automated with a macro, we can manually populate the PropertiesCoproduct
//     with all necessary types