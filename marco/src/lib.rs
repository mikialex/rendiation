extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_derive(BindGroup)]
pub fn derive_lens(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    derive_bindgroup_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

use quote::quote;
use syn::{spanned::Spanned, Data};

pub(crate) fn derive_bindgroup_impl(
    input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &input.data {
        Data::Struct(_) => derive_struct(&input),
        Data::Enum(e) => Err(syn::Error::new(
            e.enum_token.span(),
            "Bindgroup implementations cannot be derived from enums",
        )),
        Data::Union(u) => Err(syn::Error::new(
            u.union_token.span(),
            "Bindgroup implementations cannot be derived from unions",
        )),
    }
}


fn derive_struct(input: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    todo!()
}