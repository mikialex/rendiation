use crate::utils::StructInfo;
use quote::{format_ident, quote};

pub fn derive_geometry_impl(input: syn::DeriveInput) -> proc_macro2::TokenStream {
  let s = StructInfo::new(&input);
  let struct_name = &s.struct_name;
  let shadergraph_instance_name = format_ident!("{}ShaderGraphInstance", struct_name);

  let instance_fields = s.map_fields(|(field_name, ty)| {
    quote! { pub #field_name: rendiation_shadergraph::Node< #ty >, }
  });

  let instance_new = s.map_fields(|(field_name, ty)| {
    let field_str = format!("{}", field_name);
    quote! { #field_name: builder.attribute::<#ty>(#field_str), }
  });

  quote! {
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

  }
}
