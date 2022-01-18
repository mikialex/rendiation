use proc_macro::TokenStream;

use syn::parse_macro_input;

mod geometry;
mod glsl_impl;
mod ubo;
mod utils;
use geometry::*;
use glsl_impl::*;
use ubo::*;

#[proc_macro_derive(ShaderGeometry)]
pub fn derive_geometry(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_geometry_impl(input).into()
}

#[proc_macro_derive(ShaderUniformBuffer)]
pub fn derive_ubo(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_ubo_impl(&input).into()
}

#[proc_macro]
pub fn glsl_function(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::LitStr);
  let glsl = input.value();
  gen_glsl_function(&glsl, false, "").into()
}
