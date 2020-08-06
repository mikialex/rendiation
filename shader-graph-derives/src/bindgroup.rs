extern crate proc_macro;

use crate::utils::only_named_struct_fields;
use quote::{format_ident, quote};

pub fn derive_bindgroup_impl(
  input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
  let struct_name = &input.ident;
  let shadergraph_instance_name = format_ident!("{}ShaderGraphInstance", struct_name);

  let fields = only_named_struct_fields(&input)?;

  let instance_fields: Vec<_> = fields
  .iter()
  .map(|f| {
    let field_name = f.ident.as_ref().unwrap();
    let ty = &f.ty;

    quote! { pub #field_name: < #ty as rendiation_shadergraph::ShaderGraphBindGroupItemProvider>::ShaderGraphBindGroupItemInstance, }
  })
  .collect();

  let instance_new: Vec<_> = fields
  .iter()
  .map(|f| {
    let field_name = f.ident.as_ref().unwrap();
    let ty = &f.ty;
    let field_str = format!("\"{}\"", field_name);
    quote! { #field_name:< #ty as rendiation_shadergraph::ShaderGraphBindGroupItemProvider>::create_instance(#field_str, bindgroup_builder), }
  })
  .collect();

  let result = quote! {

    pub struct #shadergraph_instance_name {
      #(#instance_fields)*
    }

    impl rendiation_shadergraph::ShaderGraphBindGroupProvider for #struct_name {
      type ShaderGraphBindGroupInstance = #shadergraph_instance_name;
      fn create_instance<'a>(
        name: &'static str,
        bindgroup_builder: &mut rendiation_shadergraph::ShaderGraphBuilder<'a>)
       -> Self::ShaderGraphBindGroupInstance {
        Self::ShaderGraphUniformBufferInstance {
          #(#instance_new)*
        }
      }
    }

  };

  Ok(result)
}
