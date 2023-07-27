use quote::TokenStreamExt;
use quote::{format_ident, quote};
use shader_derives_shared::gen_struct_meta_name;

use crate::utils::StructInfo;

pub fn derive_shader_struct_impl(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let s = StructInfo::new(input);
  let mut generated = proc_macro2::TokenStream::new();
  generated.append_all(derive_shader_struct(&s));
  generated
}

fn derive_shader_struct(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;
  let shadergraph_instance_name = format_ident!("{}ShaderGraphInstance", struct_name);

  let struct_name_str = format!("{struct_name}");
  let meta_info_name = gen_struct_meta_name(struct_name_str.as_str());

  let meta_info_fields = s.map_visible_fields(|(field_name, ty)| {
    let field_str = format!("{field_name}");
    quote! {
     shadergraph::ShaderStructFieldMetaInfo {
       name: #field_str,
       ty: <<#ty as shadergraph::ShaderFieldTypeMapper>::ShaderType as shadergraph::ShaderStructMemberValueNodeType>::MEMBER_TYPE,
       ty_deco: None,
     },
    }
  });

  let instance_fields = s.map_visible_fields(|(field_name, ty)| {
    quote! { pub #field_name: shadergraph::Node<<#ty as shadergraph::ShaderFieldTypeMapper>::ShaderType>, }
  });

  let instance_fields_create = s.map_visible_fields(|(field_name, ty)| {
    let field_str = format!("{field_name}");
    quote! { #field_name: shadergraph::expand_single::<<#ty as shadergraph::ShaderFieldTypeMapper>::ShaderType>(node.handle(), #field_str), }
  });

  let construct_nodes = s.map_visible_fields(|(field_name, _ty)| {
    quote! { instance.#field_name.handle(), }
  });

  quote! {
    #[allow(non_upper_case_globals)]
    pub const #meta_info_name: &shadergraph::ShaderStructMetaInfo =
        &shadergraph::ShaderStructMetaInfo {
          name: #struct_name_str,
          fields: &[
            #(#meta_info_fields)*
          ]
        };

    #[derive(Copy, Clone)]
    pub struct #shadergraph_instance_name {
      #(#instance_fields)*
    }

    impl shadergraph::ShaderGraphNodeType for #struct_name {
      const TYPE: shadergraph::ShaderValueType =
        shadergraph::ShaderValueType::Single(<Self as shadergraph::ShaderGraphNodeSingleType>::SINGLE_TYPE);
    }

    impl shadergraph::ShaderGraphNodeSingleType for #struct_name {
      const SINGLE_TYPE: shadergraph::ShaderValueSingleType =
        shadergraph::ShaderValueSingleType::Fixed(shadergraph::ShaderStructMemberValueType::Struct(&#meta_info_name));
    }


    impl shadergraph::ShaderFieldTypeMapper for #struct_name {
      type ShaderType = #struct_name;
    }

    impl shadergraph::ShaderStructMemberValueNodeType for #struct_name {
      const MEMBER_TYPE: shadergraph::ShaderStructMemberValueType =
        shadergraph::ShaderStructMemberValueType::Struct(&#meta_info_name);
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
      fn construct(instance: Self::Instance) -> shadergraph::Node<Self>{
          shadergraph::ShaderGraphNodeExpr::StructConstruct {
            meta: Self::meta_info(),
            fields: vec![
              #(#construct_nodes)*
            ],
          }.insert_graph()
      }
    }

    impl #shadergraph_instance_name {
      pub fn construct(self) -> shadergraph::Node<#struct_name> {
        <#struct_name as shadergraph::ShaderGraphStructuralNodeType>::construct(self)
      }
    }

  }
}
