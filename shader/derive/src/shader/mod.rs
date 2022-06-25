mod glsl_function;
pub use glsl_function::*;

mod wgsl_function;
pub use wgsl_function::*;

use quote::format_ident;

pub fn gen_fn_meta_name(name: &str) -> syn::Ident {
  format_ident!("{}_SHADER_FUNCTION", name)
}
