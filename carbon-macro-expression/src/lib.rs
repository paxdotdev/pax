extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn;
use syn::DeriveInput;

#[proc_macro]
pub fn expression(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast : DeriveInput = syn::parse(input).unwrap();

    // Build the trait implementation
    // impl_hello_macro(&ast)
    impl_expression(&ast)
}

fn impl_expression(ast: &DeriveInput) -> TokenStream {
    unimplemented!()
}