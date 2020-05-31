use proc_macro2::{Literal, Span, TokenStream};
use proc_macro_roids::FieldExt;
use quote::quote;
use syn::{Data, DataStruct, DataEnum, DeriveInput, Generics, Ident, parse_quote};

pub fn impl_redirect(ast: &DeriveInput) -> TokenStream {
    let base = &ast.ident;
    let implement = match &ast.data {
        Data::Struct(ref data) => redirect_struct(base, data),
        Data::Enum(ref data) => redirect_enum(base, data),
        _ => panic!("Redirect derive only supports structs and enums"),
    };

    let (_, ty_generics, where_clause) = ast.generics.split_for_impl();
    let lf_tokens = gen_def_lt_tokens(&ast.generics);
    let ty_tokens = gen_def_ty_params(&ast.generics);

    quote! {
        impl<#lf_tokens #ty_tokens> Redirect<String, usize> for #base #ty_generics #where_clause {
            fn redirect<F>(self, map: &F) -> Self where F: Fn(String) -> usize {
                #implement
            }
        }
    }
}

fn redirect_struct(base: &Ident, data: &DataStruct) -> TokenStream {
    let ref namespace = parse_quote!(redirect);
    let ref tag = parse_quote!(skip);

    let fields = data.fields
        .iter()
        .filter(|field| !field.contains_tag(namespace, tag))
        .enumerate()
        .map(|(field_number, field)| match &field.ident {
            None => {
                let var_name = Ident::new(&format!("field_{}", field_number), Span::call_site());
                let number = Literal::usize_unsuffixed(field_number);
                quote! { #number: self.#var_name.redirect(map) }
            }
            Some(name) => quote! { #name: self.#name.redirect(map) },
        });

    quote! { #base { #(#fields),*, .. self } }
}

fn redirect_enum(_base: &Ident, _data: &DataEnum) -> TokenStream {
    /*
    let ref namespace = parse_quote!(redirect);
    let ref tag = parse_quote!(skip);
    for ref variant in data.variants {
        let fields = variant.fields
            .iter()
            .filter(|field| !field.contains_tag(namespace, tag))
            .enumerate()
            .map(|(field_number, field)| match &field.ident {
                None => {
                    let var_name = Ident::new(&format!("field_{}", field_number), Span::call_site());
                    let number = Literal::usize_unsuffixed(field_number);
                    quote! { #number: self.#var_name.redirect(map) }
                }
                Some(name) => quote! { #name: self.#name.redirect(map) },
            });
    }
     */

    unimplemented!();
}

fn gen_def_lt_tokens(generics: &Generics) -> TokenStream {
    let lts: Vec<_> = generics
        .lifetimes()
        .map(|x| {
            let lt = &x.lifetime;
            let bounds = &x.bounds;

            if bounds.is_empty() {
                quote! { #lt }
            } else {
                let bounds_iter = bounds.iter();
                quote! { #lt: #( #bounds_iter )+* }
            }
        })
        .collect();

    quote! { #( #lts ),* }
}

fn gen_def_ty_params(generics: &Generics) -> TokenStream {
    let ty_params: Vec<_> = generics
        .type_params()
        .map(|x| {
            let ty = &x.ident;
            let bounds = &x.bounds;
            let bounds_iter = bounds.iter();

            quote! { #ty: #( #bounds_iter )+* }
        })
        .collect();

    quote! { #( #ty_params ),* }
}