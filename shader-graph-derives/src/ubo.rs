extern crate proc_macro;

use crate::utils::only_named_struct_fields;
use quote::{format_ident, quote};

pub fn derive_ubo_impl(input: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
  let struct_name = &input.ident;
  let shadergraph_instance_name = format_ident!("{}ShaderGraphInstance", struct_name);

  let fields = only_named_struct_fields(input)?;

  let instance_fields: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      let ty = &f.ty;
      quote! { pub #field_name: rendiation_shadergraph::ShaderGraphNodeHandle< #ty >, }
    })
    .collect();

  let instance_new: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      let ty = &f.ty;
      let field_str = format!("\"{}\"", field_name);
      quote! { #field_name: bindgroup_builder.uniform::<#ty>(#field_str), }
    })
    .collect();

  let result = quote! {

    pub struct #shadergraph_instance_name {
      #(#instance_fields)*
    }

    impl rendiation_shadergraph::ShaderGraphBindGroupItemProvider for #struct_name {
      type ShaderGraphBindGroupItemInstance = #shadergraph_instance_name;
      fn create_instance<'a>(
        name: &'static str, // uniform buffer group not need set name
        bindgroup_builder: &mut rendiation_shadergraph::ShaderGraphBindGroupBuilder<'a>)
       -> Self::ShaderGraphBindGroupItemInstance {
        Self::ShaderGraphBindGroupItemInstance {
          #(#instance_new)*
        }
      }
    }

  };

  Ok(result)
}
