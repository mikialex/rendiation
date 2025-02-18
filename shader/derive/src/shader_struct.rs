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
          rendiation_shader_api::index_access_field::<<#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType>(node.handle(), #idx)
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
    let fn_name = format_ident!("ptr_{}", field_name);
    quote! {
      pub fn #fn_name(&self) -> rendiation_shader_api::ShaderAccessorOf<<#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType, Ptr> {
        let v = self.0.field_index(#idx);
        <#ty as rendiation_shader_api::ShaderFieldTypeMapper>::ShaderType::create_accessor_from_raw_ptr(v)
      }
    }
  });

  quote! {
    #[derive(Copy, Clone)]
    pub struct #shader_api_instance_name {
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

    pub struct #shader_api_ptr_instance_name<Ptr>(Ptr);
    impl<Ptr: Clone> Clone for #shader_api_ptr_instance_name<Ptr>{
      fn clone(&self) -> Self {
        #shader_api_ptr_instance_name(self.0.clone())
      }
    }
    impl<Ptr: Copy> Copy for #shader_api_ptr_instance_name<Ptr>{}
    impl<Ptr: rendiation_shader_api::AbstractShaderPtr> rendiation_shader_api::ShaderValueAbstractPtrAccess<Ptr> for #struct_name {
      type Accessor = #shader_api_ptr_instance_name<Ptr>;
      fn create_accessor_from_raw_ptr(ptr: Ptr) -> Self::Accessor {
        #shader_api_ptr_instance_name(ptr)
      }
    }
    impl<Ptr: rendiation_shader_api::AbstractShaderPtr> rendiation_shader_api::SizedValueShaderPtrAccessor for #shader_api_ptr_instance_name<Ptr> {
      type Node = #struct_name;
      fn load(&self) -> Node<#struct_name> {
        unsafe { self.0.load().into_node() }
      }
      fn store(&self, value: Node<#struct_name>) {
        self.0.store(value.handle());
      }
    }
    impl<Ptr: rendiation_shader_api::AbstractShaderPtr> #shader_api_ptr_instance_name<Ptr> {
      #(#ptr_sub_fields_accessor)*
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

pub fn derive_shader_struct_storage_ptr_access_impl(
  input: &syn::DeriveInput,
) -> proc_macro2::TokenStream {
  let s = StructInfo::new(input);
  let mut generated = proc_macro2::TokenStream::new();
  generated.append_all(derive_shader_struct_storage_ptr_access(&s));
  generated
}

fn derive_shader_struct_storage_ptr_access(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;

  let field_ptr_access = s
    .iter_visible_fields()
    .enumerate()
    .map(|(idx, (field_name, ty))| {
      let fn_name = format_ident!("storage_node_{}_field_ptr", field_name);
      quote! {
       pub fn #fn_name(node: rendiation_shader_api::StorageNode<#struct_name>) -> rendiation_shader_api::StorageNode<#ty> {
          unsafe { rendiation_shader_api::index_access_field(node.handle(), #idx) }
       }
      }
    })
    .collect::<Vec<_>>();

  let readonly_field_ptr_access = s
    .iter_visible_fields()
    .enumerate()
    .map(|(idx, (field_name, ty))| {
      let fn_name = format_ident!("readonly_storage_node_{}_field_ptr", field_name);
      quote! {
       pub fn #fn_name(node: rendiation_shader_api::ReadOnlyStorageNode<#struct_name>) -> rendiation_shader_api::ReadOnlyStorageNode<#ty> {
          unsafe { rendiation_shader_api::index_access_field(node.handle(), #idx) }
       }
      }
    })
    .collect::<Vec<_>>();

  quote! {
    impl #struct_name {
      #(#field_ptr_access)*
    }

    impl #struct_name {
      #(#readonly_field_ptr_access)*
    }
  }
}

pub fn derive_shader_struct_uniform_ptr_access_impl(
  input: &syn::DeriveInput,
) -> proc_macro2::TokenStream {
  let s = StructInfo::new(input);
  let mut generated = proc_macro2::TokenStream::new();
  generated.append_all(derive_shader_struct_uniform_ptr_access(&s));
  generated
}

fn derive_shader_struct_uniform_ptr_access(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;

  let field_ptr_access = s
    .iter_visible_fields()
    .enumerate()
    .map(|(idx, (field_name, ty))| {
      let fn_name = format_ident!("uniform_node_{}_field_ptr", field_name);
      quote! {
       pub fn #fn_name(node: rendiation_shader_api::UniformNode<#struct_name>) -> rendiation_shader_api::UniformNode<#ty> {
          unsafe { rendiation_shader_api::index_access_field(node.handle(), #idx) }
       }
      }
    })
    .collect::<Vec<_>>();

  quote! {
    impl #struct_name {
      #(#field_ptr_access)*
    }
  }
}
