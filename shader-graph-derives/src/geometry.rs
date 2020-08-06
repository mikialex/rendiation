use crate::utils::only_named_struct_fields;
use quote::{format_ident, quote};

pub fn derive_geometry_impl(
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
    quote! { pub #field_name: rendiation_shadergraph::ShaderGraphNodeHandle< #ty >, }
  })
  .collect();

  let instance_new: Vec<_> = fields
  .iter()
  .map(|f| {
    let field_name = f.ident.as_ref().unwrap();
    let ty = &f.ty;
    let field_str = format!("\"{}\"", field_name);
    quote! { #field_name: builder.attribute::<#ty>(#field_str), }
  })
  .collect();

  let result = quote! {

    pub struct #shadergraph_instance_name {
      #(#instance_fields)*
    }

    impl rendiation_shadergraph::ShaderGraphGeometryProvider for #struct_name {
      type ShaderGraphGeometryInstance = #shadergraph_instance_name;
      fn create_instance(
        builder: &mut rendiation_shadergraph::ShaderGraphBuilder)
       -> Self::ShaderGraphGeometryInstance {
        Self::ShaderGraphGeometryInstance {
          #(#instance_new)*
        }
      }
    }

  };

  Ok(result)
}
