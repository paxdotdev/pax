extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn pax(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let _ = input;

    input
}