use quote::format_ident;

pub fn gen_fn_meta_name(name: &str) -> syn::Ident {
  format_ident!("{}_SHADER_FUNCTION", name)
}

pub fn gen_struct_meta_name(name: &str) -> syn::Ident {
  format_ident!("{}_META_INFO", name)
}
