// use crate::shader::gen_meta_name;
use quote::quote;
use wgsl_parser::*;

pub fn gen_wgsl_function(wgsl: &str) -> proc_macro2::TokenStream {
  FunctionDefine::parse_input(wgsl).unwrap();
  quote! {}
}
