use proc_macro2::{Literal, Span, TokenStream};
use proc_macro_roids::{FieldExt, contains_tag};
use quote::quote;
use syn::{Data, DataStruct, DataEnum, DeriveInput, Generics, Ident, parse_quote, Fields, Path};

pub fn impl_redirect(ast: &DeriveInput) -> TokenStream {
    let namespace = parse_quote!(redirect);
    let tag = parse_quote!(skip);

    let base = &ast.ident;
    let implement = match &ast.data {
        Data::Struct(ref data) => redirect_struct(base, data, &namespace, &tag),
        Data::Enum(ref data) => redirect_enum(base, data, &namespace, &tag),
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

fn redirect_struct(
    base: &Ident,
    data: &DataStruct,
    namespace: &Path,
    tag: &Path,
) -> TokenStream {
    let extract = extract_fields(base, &data.fields, namespace, tag);
    let fields = redirect_fields(&data.fields, namespace, tag);
    quote! { #extract #base { #(#fields),*, ..self } }
}

fn redirect_enum(
    base: &Ident,
    data: &DataEnum,
    namespace: &Path,
    tag: &Path,
) -> TokenStream {
    let variants = data.variants
        .iter()
        .filter(|variant| !contains_tag(variant.attrs.as_slice(), namespace, tag))
        .map(|variant| {
            let variant_name = &variant.ident;
            let field_names = field_names(&variant.fields, namespace, tag);
            let fields = redirect_fields(&variant.fields, namespace, tag);
            quote! { #(#base::#variant_name ( #field_names ) => #base::#variant_name { #fields }),* }
        });

    if variants.clone().count() < data.variants.len() {
        quote! { match self { #(#variants),*, _ => self } }
    } else {
        quote! { match self { #(#variants),* } }
    }.into()
}

fn extract_fields(
    base: &Ident,
    fields: &Fields,
    namespace: &Path,
    tag: &Path,
) -> TokenStream {
    let field_names = field_names(fields, namespace, tag);
    if field_names.clone().count() < fields.len() {
        quote! { let #base { #(#field_names),*, .. } = self; }
    } else {
        quote! { let #base { #(#field_names),* } = self; }
    }.into()
}

fn redirect_fields<'a>(
    fields: &'a Fields,
    namespace: &'a Path,
    tag: &'a Path,
) -> impl Iterator<Item=TokenStream> + Clone + 'a {
    fields
        .iter()
        .filter(move |field| !field.contains_tag(namespace, tag))
        .enumerate()
        .map(|(field_number, field)| match &field.ident {
            None => {
                let var_name = Ident::new(&format!("field_{}", field_number), Span::call_site());
                let number = Literal::usize_unsuffixed(field_number);
                quote! { #number: #var_name.redirect(map) }
            }
            Some(name) => quote! { #name: #name.redirect(map) },
        })
}

fn field_names<'a>(
    fields: &'a Fields,
    namespace: &'a Path,
    tag: &'a Path,
) -> impl Iterator<Item=TokenStream> + Clone + 'a {
    fields
        .iter()
        .filter(move |field| !field.contains_tag(namespace, tag))
        .enumerate()
        .map(|(field_number, field)| match &field.ident {
            None => {
                let var_name = Ident::new(&format!("field_{}", field_number), Span::call_site());
                quote! { #var_name }
            }
            Some(name) => quote! { #name },
        })
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