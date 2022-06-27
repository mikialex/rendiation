use proc_macro::TokenStream;
use syn::parse_macro_input;

// mod shader;
mod shader_struct;
mod std140;
mod utils;
mod vertex;
// use shader::*;
use shader_struct::*;
use std140::*;
use vertex::*;

/// Mark the struct could be used as vertex input type in shadergraph
///
/// The struct's mem layout will generate the correct vertex buffer layout
/// and inject semantic shader value in shadergraph.
#[proc_macro_derive(ShaderVertex, attributes(semantic))]
pub fn derive_vertex(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_vertex_impl(input).into()
}

/// Mark the struct could be expressed in shadergraph type API
///
/// Implementation will add static struct meta info for reflection
/// and define a shader graph instance type and convert methods for shadergraph usage.
#[proc_macro_derive(ShaderStruct)]
pub fn derive_shader_struct(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_shader_struct_impl(&input).into()
}

// /// Create shadergraph function by parsing glsl source code.
// #[proc_macro]
// pub fn glsl_function(input: TokenStream) -> TokenStream {
//   let input = format!("{}", input);
//   gen_glsl_function(input.as_str()).into()
// }

/// Validate the struct if possible to create std140 memory layout version.
///
/// Convert the struct into std140 version by type mapping and insert correct paddings between fields
#[proc_macro_attribute]
pub fn std140_layout(_args: TokenStream, input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  let expanded = std140_impl(input);

  TokenStream::from(expanded)
}
