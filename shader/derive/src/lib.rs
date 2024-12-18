use proc_macro::TokenStream;
use syn::parse_macro_input;

mod shader_align;
mod shader_fn;
mod shader_struct;
mod utils;
mod vertex;
use shader_align::*;
use shader_fn::*;
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

/// Generate field access methods for StorageNode<StructType>
#[proc_macro_derive(StorageNodePtrAccess)]
pub fn derive_shader_struct_storage_ptr_access(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_shader_struct_storage_ptr_access_impl(&input).into()
}
/// Generate field access methods for UniformNode<StructType>
#[proc_macro_derive(UniformNodePtrAccess)]
pub fn derive_shader_struct_uniform_ptr_access(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_shader_struct_uniform_ptr_access_impl(&input).into()
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
/// one for bool and mat3, but we don't, because this will cause too many confusion in users's code.
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
/// one for bool and mat3, but we don't, because this will cause too many confusion in users's code.
#[proc_macro_attribute]
pub fn std430_layout(_args: TokenStream, input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  let expanded = shader_align_gen(input, "Std430", 0);

  TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn shader_fn(_args: TokenStream, input: TokenStream) -> TokenStream {
  shader_api_fn_impl(_args, input)
}
