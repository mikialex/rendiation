use proc_macro::TokenStream;

use syn::parse_macro_input;

mod glsl_impl;
mod std140;
mod uniform;
mod utils;
mod vertex;
use glsl_impl::*;
use std140::*;
use uniform::*;
use vertex::*;

#[proc_macro_derive(ShaderVertex, attributes(semantic))]
pub fn derive_vertex(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_vertex_impl(input).into()
}

#[proc_macro_derive(ShaderUniform)]
pub fn derive_ubo(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_ubo_impl(&input).into()
}

#[proc_macro]
pub fn glsl_function(input: TokenStream) -> TokenStream {
  let input = format!("{}", input);
  gen_glsl_function(input.as_str()).into()
}

#[proc_macro_attribute]
pub fn std140_layout(_args: TokenStream, input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  let expanded = std140_impl(input);

  TokenStream::from(expanded)
}
