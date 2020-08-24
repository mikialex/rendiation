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
    let field_str = format!("{}", field_name);
    let attr = f.attrs.iter().find(|a| a.path.is_ident("stage")).unwrap();
    let name = format!("{}", attr.tokens); // can i do better?
    let visibility = match name.as_str() {
      "(vert)" => quote! { rendiation_shadergraph::ShaderStage::Vertex },
      "(frag)" => quote! { rendiation_shadergraph::ShaderStage::Fragment },
      _ => panic!("unsupported"),
    };

    quote! { #field_name:<#ty as rendiation_shadergraph::ShaderGraphBindGroupItemProvider>::create_instance(#field_str, bindgroup_builder, #visibility), }
  })
  .collect();

  let wgpu_create_bindgroup_fn_param: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      let ty = &f.ty;
      quote! { #field_name: < #ty as rendiation_shadergraph::WGPUBindgroupItem>::Type, }
    })
    .collect();

  let wgpu_create_bindgroup_create: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      let ty = &f.ty;
      quote! {.push(<#ty as rendiation_shadergraph::WGPUBindgroupItem>::to_binding(#field_name))}
    })
    .collect();


  let wgpu_create_bindgroup_layout_create: Vec<_> = fields
    .iter()
    .map(|f| {
      let ty = &f.ty;
      let attr = f.attrs.iter().find(|a| a.path.is_ident("stage")).unwrap();
      let name = format!("{}", attr.tokens); // can i do better?
      let visibility = match name.as_str() {
        "(vert)" => quote! { rendiation_webgpu::ShaderStage::VERTEX },
        "(frag)" => quote! { rendiation_webgpu::ShaderStage::FRAGMENT },
        _ => panic!("unsupported"),
      };

      quote! {.bind(
       <#ty as rendiation_shadergraph::WGPUBindgroupItem>::to_layout_type(),
       #visibility
      )}
    })
    .collect();

  let result = quote! {
    impl rendiation_webgpu::BindGroupProvider for #struct_name {

      fn provide_layout(renderer: &rendiation_webgpu::WGPURenderer) -> rendiation_webgpu::BindGroupLayout {
        rendiation_webgpu::BindGroupLayoutBuilder::new()
        #(#wgpu_create_bindgroup_layout_create)*
        .build(renderer)
      }
    }

    impl #struct_name {
      pub fn create_bindgroup(
        renderer: &rendiation_webgpu::WGPURenderer,
        #(#wgpu_create_bindgroup_fn_param)*
      ) -> rendiation_webgpu::WGPUBindGroup{

        rendiation_webgpu::BindGroupBuilder::new()
          #(#wgpu_create_bindgroup_create)*
          .build(
            &renderer.device,
            renderer.bindgroup_layout_cache.borrow().get(&std::any::TypeId::of::<#struct_name>())
            .expect("bindgroup need register into renderer before use")
          )

      }

    }

    pub struct #shadergraph_instance_name {
      #(#instance_fields)*
    }

    impl rendiation_shadergraph::ShaderGraphBindGroupProvider for #struct_name {
      type ShaderGraphBindGroupInstance = #shadergraph_instance_name;
      fn create_instance<'a>(
        bindgroup_builder: &mut rendiation_shadergraph::ShaderGraphBindGroupBuilder<'a>)
       -> Self::ShaderGraphBindGroupInstance {
        Self::ShaderGraphBindGroupInstance {
          #(#instance_new)*
        }
      }
    }

  };

  Ok(result)
}
