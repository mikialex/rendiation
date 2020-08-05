extern crate proc_macro;

use syn::{spanned::Spanned, Data};
use quote::{format_ident, quote};

pub(crate) fn derive_ubo_impl(
  input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
  match &input.data {
    Data::Struct(_) => derive_struct(&input),
    Data::Enum(e) => Err(syn::Error::new(
      e.enum_token.span(),
      "UniformBuffer implementations cannot be derived from enums",
    )),
    Data::Union(u) => Err(syn::Error::new(
      u.union_token.span(),
      "UniformBuffer implementations cannot be derived from unions",
    )),
  }
}

fn derive_struct(input: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
  let struct_name = &input.ident;

  let shadergraph_instance_name = format_ident!("{}ShaderGraphInstance", struct_name);

  let fields = if let syn::Data::Struct(syn::DataStruct {
    fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
    ..
  }) = input.data
  {
    named
  } else {
    return Err(syn::Error::new(
      input.span(),
      "Uniform implementations can only be derived from structs with named fields",
    ));
  };

  let instance_fields: Vec<_> = fields.iter().map(|f| {
    let field_name = f.ident.as_ref().unwrap();
    let ty = &f.ty;
    quote! { #field_name: rendiation_shadergraph::ShaderGraphNodeHandle< #ty >, }
  }).collect();

  let instance_new: Vec<_> = fields.iter().map(|f| {
    let field_name = f.ident.as_ref().unwrap();
    let ty = &f.ty;
    let field_str = format!("\"{}\"", field_name);
    quote! { #field_name: bindgroup_builder.uniform::<#ty>(#field_str), }
  }).collect();

  let result = quote! {

    pub struct #shadergraph_instance_name {
      #(#instance_fields)*
    }


    impl rendiation_shadergraph::ShaderGraphUniformBuffer for #struct_name {
      type ShaderGraphUniformBufferInstance = #shadergraph_instance_name;
      fn create_instance<'a>(bindgroup_builder: &mut rendiation_shadergraph::ShaderGraphBindGroupBuilder<'a>)
       -> Self::ShaderGraphUniformBufferInstance {
        Self::ShaderGraphUniformBufferInstance {
          #(#instance_new)*
        }
      }
    }
    
  };

  Ok(result)
}
