use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod redirect;

#[proc_macro_derive(Redirect, attributes(redirect))]
pub fn redirect_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let gen = redirect::impl_redirect(&ast);
    gen.into()
}