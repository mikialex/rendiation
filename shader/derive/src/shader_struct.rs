use quote::TokenStreamExt;
use quote::{format_ident, quote};

pub fn gen_struct_meta_name(name: &str) -> syn::Ident {
  format_ident!("{}_META_INFO", name)
}

use crate::utils::StructInfo;

pub fn derive_shader_struct_impl(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let s = StructInfo::new(input);
  let mut generated = proc_macro2::TokenStream::new();
  generated.append_all(derive_shader_struct(&s));
  generated
}

fn derive_shader_struct(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;
  let shader_api_instance_name = format_ident!("{}ShaderAPIInstance", struct_name);

  let struct_name_str = format!("{struct_name}");
  let meta_info_name = gen_struct_meta_name(struct_name_str.as_str());

  let meta_info_fields = s.map_visible_fields(|(field_name, ty)| {
    let field_str = format!("{field_name}");
    quote! {
     rendiation_shader_api::ShaderStructFieldMetaInfo {
       name: #field_str,
       ty: <<#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType as rendiation_shader_api::ShaderSizedValueNodeType>::MEMBER_TYPE,
       ty_deco: None,
     },
    }
  });

  let mut i = 0;
  let field_methods = s.map_visible_fields(|(field_name, ty)| {
    i += 1;
    let idx: usize = i - 1;
    quote! {
      pub fn #field_name(node: rendiation_shader_api::Node<Self>) -> rendiation_shader_api::Node<<#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType> {
        unsafe {
          rendiation_shader_api::expand_single::<<#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType>(node.handle(), #idx)
        }
      }
    }
  });

  let instance_fields = s.map_visible_fields(|(field_name, ty)| {
    quote! { pub #field_name: rendiation_shader_api::Node<<#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType>, }
  });

  let instance_fields_create = s.map_visible_fields(|(field_name, _ty)| {
    quote! { #field_name: Self::#field_name(node), }
  });

  let construct_nodes = s.map_visible_fields(|(field_name, _ty)| {
    quote! { instance.#field_name.handle(), }
  });

  quote! {
    #[allow(non_upper_case_globals)]
    pub const #meta_info_name: &rendiation_shader_api::ShaderStructMetaInfo =
        &rendiation_shader_api::ShaderStructMetaInfo {
          name: #struct_name_str,
          fields: &[
            #(#meta_info_fields)*
          ]
        };

    #[derive(Copy, Clone)]
    pub struct #shader_api_instance_name {
      #(#instance_fields)*
    }

    impl rendiation_shader_api::ShaderNodeType for #struct_name {
      const TYPE: rendiation_shader_api::ShaderValueType =
        rendiation_shader_api::ShaderValueType::Single(<Self as rendiation_shader_api::ShaderNodeSingleType>::SINGLE_TYPE);
    }

    impl rendiation_shader_api::ShaderNodeSingleType for #struct_name {
      const SINGLE_TYPE: rendiation_shader_api::ShaderValueSingleType =
        rendiation_shader_api::ShaderValueSingleType::Sized(rendiation_shader_api::ShaderSizedValueType::Struct(&#meta_info_name));
    }

    impl #struct_name {
      #(#field_methods)*
    }

    impl rendiation_shader_api::ShaderFieldTypeMapper for #struct_name {
      type ShaderType = #struct_name;
    }

    impl rendiation_shader_api::ShaderSizedValueNodeType for #struct_name {
      const MEMBER_TYPE: rendiation_shader_api::ShaderSizedValueType =
        rendiation_shader_api::ShaderSizedValueType::Struct(&#meta_info_name);
    }

    impl rendiation_shader_api::ShaderStructuralNodeType for #struct_name {
      type Instance = #shader_api_instance_name;
      fn meta_info() -> &'static rendiation_shader_api::ShaderStructMetaInfo{
        &#meta_info_name
      }
      fn expand(node: rendiation_shader_api::Node<Self>) -> Self::Instance{
        #shader_api_instance_name{
          #(#instance_fields_create)*
        }
      }
      fn construct(instance: Self::Instance) -> rendiation_shader_api::Node<Self>{
          rendiation_shader_api::ShaderNodeExpr::StructConstruct {
            meta: Self::meta_info(),
            fields: vec![
              #(#construct_nodes)*
            ],
          }.insert_api()
      }
    }

    impl #shader_api_instance_name {
      pub fn construct(self) -> rendiation_shader_api::Node<#struct_name> {
        <#struct_name as rendiation_shader_api::ShaderStructuralNodeType>::construct(self)
      }
    }

    impl From<#shader_api_instance_name> for rendiation_shader_api::Node<#struct_name>{
      fn from(v: #shader_api_instance_name) -> Self {
        v.construct()
      }
    }

  }
}
