use quote::TokenStreamExt;
use quote::{format_ident, quote};

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

  let meta_info_fields = s.map_collect_visible_fields(|(field_name, ty)| {
    let field_str = format!("{field_name}");
    quote! {
      .add_field::<<#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType>(#field_str)
    }
  });

  let mut i = 0;
  let field_methods = s.map_collect_visible_fields(|(field_name, ty)| {
    i += 1;
    let idx: usize = i - 1;
    quote! {
      pub fn #field_name(node: rendiation_shader_api::Node<Self>) -> rendiation_shader_api::Node<<#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType> {
        unsafe {
          rendiation_shader_api::index_access_field(node.handle(), #idx).into_node()
        }
      }
    }
  });

  let instance_fields = s.map_collect_visible_fields(|(field_name, ty)| {
    quote! { pub #field_name: rendiation_shader_api::Node<<#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType>, }
  });

  let instance_fields_create = s.map_collect_visible_fields(|(field_name, _ty)| {
    quote! { #field_name: Self::#field_name(node), }
  });

  let construct_nodes = s.map_collect_visible_fields(|(field_name, _ty)| {
    quote! { instance.#field_name.handle(), }
  });

  let to_values = s.map_collect_visible_fields(|(field_name, _ty)| {
    quote! { self.#field_name.into_shader_ty().to_value(), }
  });

  // shader ptr part
  let shader_api_ptr_instance_name = format_ident!("{}ShaderAPIPtrInstance", struct_name);
  let mut i = 0;
  let ptr_sub_fields_accessor = s.map_collect_visible_fields(|(field_name, ty)| {
    i += 1;
    let idx: usize = i - 1;
    quote! {
      pub fn #field_name(&self) -> rendiation_shader_api::ShaderPtrOf<<#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType> {
        let v = self.0.field_index(#idx);
        <#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType::create_view_from_raw_ptr(v)
      }
    }
  });

  // shader readonly ptr part
  let shader_api_readonly_ptr_instance_name =
    format_ident!("{}ShaderAPIReadonlyPtrInstance", struct_name);
  let mut i = 0;
  let readonly_ptr_sub_fields_accessor = s.map_collect_visible_fields(|(field_name, ty)| {
    i += 1;
    let idx: usize = i - 1;
    quote! {
      pub fn #field_name(&self) -> rendiation_shader_api::ShaderReadonlyPtrOf<<#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType> {
        let v = self.0.field_index(#idx);
        <#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType::create_readonly_view_from_raw_ptr(v)
      }
    }
  });

  let struct_vis = &s.vis;

  quote! {
    #[derive(Copy, Clone)]
    #struct_vis struct #shader_api_instance_name {
      #(#instance_fields)*
    }

    impl rendiation_shader_api::ShaderNodeType for #struct_name {
      fn ty() -> rendiation_shader_api::ShaderValueType {
        rendiation_shader_api::ShaderValueType::Single(Self::single_ty())
      }
    }

    impl rendiation_shader_api::ShaderNodeSingleType for #struct_name {
      fn single_ty() -> rendiation_shader_api::ShaderValueSingleType {
        rendiation_shader_api::ShaderValueSingleType::Sized(Self::sized_ty())
      }
    }

    #[derive(Clone)]
    #struct_vis struct #shader_api_ptr_instance_name(BoxedShaderPtr);
    #[derive(Clone)]
    #struct_vis struct #shader_api_readonly_ptr_instance_name(BoxedShaderPtr);
    impl rendiation_shader_api::ShaderAbstractPtrAccess for #struct_name {
      type PtrView = #shader_api_ptr_instance_name;
      type ReadonlyPtrView = #shader_api_readonly_ptr_instance_name;
      fn create_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::PtrView {
        #shader_api_ptr_instance_name(ptr)
      }
      fn create_readonly_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyPtrView {
        #shader_api_readonly_ptr_instance_name(ptr)
      }
    }
    impl rendiation_shader_api::ReadonlySizedShaderPtrView for #shader_api_readonly_ptr_instance_name {
      type Node = #struct_name;
      fn load(&self) -> Node<#struct_name> {
        unsafe { self.0.load().into_node() }
      }
      fn raw(&self) -> &BoxedShaderPtr {
        &self.0
      }
    }
    impl rendiation_shader_api::ReadonlySizedShaderPtrView for #shader_api_ptr_instance_name {
      type Node = #struct_name;
      fn load(&self) -> Node<#struct_name> {
        unsafe { self.0.load().into_node() }
      }
      fn raw(&self) -> &BoxedShaderPtr {
        &self.0
      }
    }
    impl rendiation_shader_api::SizedShaderPtrView for #shader_api_ptr_instance_name {
      fn store(&self, value: impl Into<Node<#struct_name>>) {
        self.0.store(value.into().handle());
      }
    }
    impl #shader_api_ptr_instance_name {
      #(#ptr_sub_fields_accessor)*
    }
    impl #shader_api_readonly_ptr_instance_name {
      #(#readonly_ptr_sub_fields_accessor)*
    }

    impl #struct_name {
      #(#field_methods)*
    }

    impl rendiation_shader_api::ShaderFieldTypeMapper for #struct_name {
      type ShaderType = #struct_name;
      fn into_shader_ty(self) -> Self::ShaderType {
        self
      }
    }

    impl rendiation_shader_api::ShaderSizedValueNodeType for #struct_name {
      fn sized_ty() -> rendiation_shader_api::ShaderSizedValueType {
        rendiation_shader_api::ShaderSizedValueType::Struct(Self::meta_info())
      }
      fn to_value(&self) -> ShaderStructFieldInitValue {
        ShaderStructFieldInitValue::Struct(vec![
          #(#to_values)*
        ])
      }
    }

    impl rendiation_shader_api::ShaderStructuralNodeType for #struct_name {
      type Instance = #shader_api_instance_name;
      fn meta_info() -> rendiation_shader_api::ShaderStructMetaInfo{
        ShaderStructMetaInfo::new(#struct_name_str)
        #(#meta_info_fields)*
      }
      fn expand(node: rendiation_shader_api::Node<Self>) -> Self::Instance{
        #shader_api_instance_name{
          #(#instance_fields_create)*
        }
      }
      fn construct(instance: Self::Instance) -> rendiation_shader_api::Node<Self>{
          rendiation_shader_api::ShaderNodeExpr::Compose {
            target: ShaderSizedValueType::Struct(Self::meta_info()),
            parameters: vec![
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
