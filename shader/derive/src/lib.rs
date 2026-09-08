use proc_macro::TokenStream;
use syn::parse_macro_input;

mod shader_align;
mod shader_fn;
mod shader_name_marked;
mod shader_struct;
mod utils;
mod vertex;
use shader_align::*;
use shader_fn::*;
use shader_name_marked::*;
use shader_struct::*;
use vertex::*;

/// Mark the struct could be used as vertex input type in rendiation_shader_api
///
/// The struct's mem layout will generate the correct vertex buffer layout
/// and inject semantic shader value in rendiation_shader_api.
///
/// ## The memory layout should be compact
#[proc_macro_derive(ShaderVertex, attributes(semantic))]
pub fn derive_vertex(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_vertex_impl(input).into()
}

/// Mark the struct could be expressed in rendiation_shader_api type API
///
/// Implementation will add static struct meta info for reflection
/// and define a shader api instance type and convert methods for rendiation_shader_api usage.
#[proc_macro_derive(ShaderStruct)]
pub fn derive_shader_struct(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_shader_struct_impl(&input).into()
}

/// Validate the struct if possible to create std140 memory layout version.
///
/// Convert the struct into std140 version by type mapping and insert correct paddings between
/// fields
///
/// Note: some primitive types, like bool, Mat3<f32> have totally different memory layouts that we
/// can't insert padding into type itself. In this situation, the user should use their pre
/// converted type like Bool, Shader16PaddedMat3 instead of the original one.
///
/// The other design choice is, theoretically we could directly convert the field into the std140
/// one for bool and mat3, but we don't, because this will cause too many confusions in users' code.
#[proc_macro_attribute]
pub fn std140_layout(_args: TokenStream, input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  let expanded = shader_align_gen(input, "Std140", 16);

  TokenStream::from(expanded)
}

/// Validate the struct if possible to create std430 memory layout version.
///
/// Convert the struct into std430 version by type mapping and insert correct paddings between
/// fields
///
/// Note: some primitive types, like bool, Mat3<f32> have totally different memory layouts that we
/// can't insert padding into type itself. In this situation, the user should use their pre
/// converted type like Bool, Shader16PaddedMat3 instead of the original one.
///
/// The other design choice is, theoretically we could directly convert the field into the 430
/// one for bool and mat3, but we don't, because this will cause too many confusions in users' code.
#[proc_macro_attribute]
pub fn std430_layout(_args: TokenStream, input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  let expanded = shader_align_gen(input, "Std430", 0);

  TokenStream::from(expanded)
}

/// Mark the function callable on the GPU side, auto-deduplicated by its unique name.
///
/// Every `let` binding in the function body and every parameter is automatically debug name
/// marked with its Rust variable name, same behavior as `shader_name_marked`.
#[proc_macro_attribute]
pub fn shader_fn(_args: TokenStream, input: TokenStream) -> TokenStream {
  shader_api_fn_impl(_args, input)
}

/// Mark every `let` binding in the function body with its Rust variable name, so the name shows
/// up in generated WGSL as debug labels for the underlying shader node.
///
/// The inserted mark is a no-op for non shader node values, only `Node<T>` bindings are really
/// named, in a best effort way. `shader_fn` already applies this marking to its body, applying
/// this attribute additionally on top of it is allowed but redundant, and if combined it must be
/// written on the outer side, because `shader_fn` expansion discards the other attributes on the
/// function.
#[proc_macro_attribute]
pub fn shader_name_marked(_args: TokenStream, input: TokenStream) -> TokenStream {
  shader_name_marked_impl(_args, input)
}
