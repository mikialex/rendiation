use crate::utils::StructInfo;
use quote::TokenStreamExt;
use quote::{format_ident, quote};

pub fn derive_ubo_impl(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let s = StructInfo::new(input);
  let mut generated = proc_macro2::TokenStream::new();
  generated.append_all(derive_shader_struct(&s));
  generated
}

pub fn derive_shader_struct(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;
  let shadergraph_instance_name = format_ident!("{}ShaderGraphInstance", struct_name);

  let struct_name_str = format!("{}", struct_name);
  let meta_info_name = format_ident!("{}_META_INFO", struct_name);

  let meta_info_gen = s.map_fields(|(field_name, ty)| {
    let field_str = format!("{}", field_name);
    quote! { .add_field::<#ty>(#field_str) }
  });

  let instance_fields = s.map_fields(|(field_name, ty)| {
    quote! { pub #field_name: shadergraph::Node<#ty>, }
  });

  let instance_fields_create = s.map_fields(|(field_name, ty)| {
    let field_str = format!("{}", field_name);
    quote! { #field_name: shadergraph::expand_single::<#ty>(node.handle(), #field_str), }
  });

  quote! {
    #[allow(non_upper_case_globals)]
    pub static #meta_info_name: once_cell::sync::Lazy<shadergraph::ShaderStructMetaInfo> =
    once_cell::sync::Lazy::new(|| {
        shadergraph::ShaderStructMetaInfo::new(
          #struct_name_str,
        )
        #(#meta_info_gen)*
    });

    pub struct #shadergraph_instance_name {
      #(#instance_fields)*
    }

    impl shadergraph::ShaderGraphNodeType for #struct_name {
      fn to_type() -> shadergraph::ShaderValueType{
        shadergraph::ShaderValueType::Fixed(
          shadergraph::ShaderStructMemberValueType::Struct(&#meta_info_name)
        )
      }
    }

    impl shadergraph::ShaderStructMemberValueNodeType for #struct_name {
      fn to_type() -> shadergraph::ShaderStructMemberValueType {
        shadergraph::ShaderStructMemberValueType::Struct(&#meta_info_name)
      }
    }

    impl shadergraph::ShaderGraphStructuralNodeType for #struct_name {
      type Instance = #shadergraph_instance_name;
      fn meta_info() -> &'static shadergraph::ShaderStructMetaInfo{
        &#meta_info_name
      }
      fn expand(node: shadergraph::Node<Self>) -> Self::Instance{
        #shadergraph_instance_name{
          #(#instance_fields_create)*
        }
      }
    }

  }
}
