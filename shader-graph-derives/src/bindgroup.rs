extern crate proc_macro;

use crate::utils::only_named_struct_fields;
use quote::TokenStreamExt;
use quote::{format_ident, quote};

pub fn derive_bindgroup_impl(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let mut generated = proc_macro2::TokenStream::new();
  generated.append_all(derive_wgpu_layout(input));
  generated.append_all(derive_shadergraph_instance(input));
  generated.append_all(derive_ral_wgpu_bindgroup(input));
  generated.append_all(derive_wgpu_bindgroup_direct_create(input));
  generated.append_all(derive_webgl_upload_instance(input));
  generated
}

fn derive_webgl_upload_instance(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let struct_name = &input.ident;
  let instance_name = format_ident!("{}WebGLUniformUploadInstance", struct_name);

  let fields = only_named_struct_fields(input).unwrap();
  let fields_info: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap().clone();
      let ty = f.ty.clone();
      (field_name, ty)
    })
    .collect();

  let instance_fields: Vec<_> = fields_info
    .iter()
    .map(|(field_name, ty)| {
      quote! { pub #field_name: <#ty as rendiation_webgl::WebGLUniformUploadable>::UploadInstance, }
    })
    .collect();

  let instance_create: Vec<_> = fields_info
    .iter()
    .map(|(field_name, ty)| {
      let field_str = format!("{}", field_name);
      quote! { #field_name:
       < <#ty as rendiation_webgl::WebGLUniformUploadable>::UploadInstance
       as rendiation_webgl::UploadInstance<#ty> >::create(
          format!("{}{}", query_name_prefix, #field_str).as_str(),
          gl,
          program
       ),
      }
    })
    .collect();

  let instance_upload: Vec<_> = fields_info
    .iter()
    .map(|(field_name, ty)| {
      quote! { self.#field_name.upload(unsafe{std::mem::transmute(&<#ty as rendiation_ral::RALBindgroupItem<rendiation_webgl::WebGLRenderer>>::get_item(value.#field_name, resources))}, gl, resources); }
    })
    .collect();

  let ral_instance_name = format_ident!("{}RALInstance", struct_name);

  quote! {
    pub struct #instance_name {
      #(#instance_fields)*
    }

    impl rendiation_webgl::UploadInstance<#struct_name> for #instance_name {
      fn create(
        query_name_prefix: &str,
        gl: &rendiation_webgl::WebGl2RenderingContext,
        program: &rendiation_webgl::WebGlProgram,
      ) -> Self{
        Self {
          #(#instance_create)*
        }
      }
      fn upload(
        &mut self,
        value: &#ral_instance_name<rendiation_webgl::WebGLRenderer>,
        gl: &rendiation_webgl::WebGl2RenderingContext,
        resources: &rendiation_ral::ShaderBindableResourceManager<rendiation_webgl::WebGLRenderer>,
      ){
        #(#instance_upload)*
      }
    }

    impl rendiation_webgl::WebGLUniformUploadable for #struct_name {
      type UploadValue = <#struct_name as rendiation_ral::BindGroupProvider<rendiation_webgl::WebGLRenderer>>::Instance;
      type UploadInstance = #instance_name;
    }
  }
}

fn derive_ral_wgpu_bindgroup(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let struct_name = &input.ident;
  let fields = only_named_struct_fields(&input).unwrap();

  let ral_instance_name = format_ident!("{}RALInstance", struct_name);

  let fields_info: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap().clone();
      let ty = f.ty.clone();
      (field_name, ty)
    })
    .collect();

  let ral_fields: Vec<_> = fields_info
    .iter()
    .map(|(field_name, ty)| {
      quote! { pub #field_name: < #ty as rendiation_ral::RALBindgroupHandle<T>>::HandleType, }
    })
    .collect();

  let wgpu_resource_get: Vec<_> = fields_info
    .iter()
    .map(|(field_name, ty)| {
      quote! {let #field_name = <#ty as rendiation_ral::RALBindgroupItem<WGPURenderer>>::get_item(instance.#field_name, resources);}
    })
    .collect();

  let create_resource_instance_fn_param: Vec<_> = fields_info
    .iter()
    .map(|(field_name, ty)| {
      quote! {#field_name: < #ty as rendiation_ral::RALBindgroupHandle<T>>::HandleType,}
    })
    .collect();

  let create_resource_instance_field: Vec<_> = fields_info
    .iter()
    .map(|(field_name, _)| {
      quote! {#field_name,}
    })
    .collect();

  let wgpu_create_bindgroup_create: Vec<_> = fields_info
    .iter()
    .map(|(field_name, ty)| {
      quote! {.push(<#ty as rendiation_shadergraph::WGPUBindgroupItem>::to_binding(#field_name))}
    })
    .collect();

  quote! {
    pub struct #ral_instance_name<T: rendiation_ral::RALBackend> {
      #(#ral_fields)*
    }

    impl rendiation_ral::BindGroupCreator<rendiation_webgpu::WGPURenderer> for #struct_name {
      fn create_bindgroup(
        instance: &Self::Instance,
        renderer: &<rendiation_webgpu::WGPURenderer as rendiation_ral::RALBackend>::Renderer,
        resources: &rendiation_ral::ShaderBindableResourceManager<rendiation_webgpu::WGPURenderer>,
      ) -> <rendiation_webgpu::WGPURenderer as rendiation_ral::RALBackend>::BindGroup {
        renderer.register_bindgroup::<Self>();

         #(#wgpu_resource_get)*

        rendiation_webgpu::BindGroupBuilder::new()
          #(#wgpu_create_bindgroup_create)*
          .build(
            &renderer.device,
            renderer.bindgroup_layout_cache.borrow().get(&std::any::TypeId::of::<#struct_name>())
            .unwrap()
          )
      }
    }

    impl rendiation_ral::BindGroupCreator<rendiation_webgl::WebGLRenderer> for #struct_name {
      fn create_bindgroup(
        instance: &Self::Instance,
        renderer: &<rendiation_webgl::WebGLRenderer as rendiation_ral::RALBackend>::Renderer,
        resources: &rendiation_ral::ShaderBindableResourceManager<rendiation_webgl::WebGLRenderer>,
      ) -> <rendiation_webgl::WebGLRenderer as rendiation_ral::RALBackend>::BindGroup {
        ()
      }
    }

    impl<T: rendiation_ral::RALBackend> rendiation_ral::BindGroupProvider<T> for #struct_name {
      type Instance =  #ral_instance_name<T>;

      fn apply(
        instance: &Self::Instance,
        gpu_bindgroup: &T::BindGroup,
        index: usize,
        shading: &T::Shading,
        resources: &rendiation_ral::ShaderBindableResourceManager<T>,
        render_pass: &mut T::RenderPass,
      ){
        T::apply_bindgroup(render_pass, index, gpu_bindgroup);
      }
    }

    impl #struct_name {
      pub fn create_resource_instance<T: rendiation_ral::RALBackend>(
        #(#create_resource_instance_fn_param)*
      ) ->  #ral_instance_name<T> {
        #ral_instance_name {
          #(#create_resource_instance_field)*
        }
      }
    }

  }
}

fn derive_wgpu_bindgroup_direct_create(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let struct_name = &input.ident;
  let fields = only_named_struct_fields(&input).unwrap();

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

  quote! {

    impl #struct_name {
      pub fn create_bindgroup(
        renderer: &rendiation_webgpu::WGPURenderer,
        #(#wgpu_create_bindgroup_fn_param)*
      ) -> rendiation_webgpu::WGPUBindGroup {

        renderer.register_bindgroup::<Self>();

        rendiation_webgpu::BindGroupBuilder::new()
          #(#wgpu_create_bindgroup_create)*
          .build(
            &renderer.device,
            renderer.bindgroup_layout_cache.borrow().get(&std::any::TypeId::of::<#struct_name>())
            .unwrap()
          )

      }

    }

  }
}

fn derive_shadergraph_instance(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let struct_name = &input.ident;
  let fields = only_named_struct_fields(&input).unwrap();

  let shadergraph_instance_name = format_ident!("{}ShaderGraphInstance", struct_name);

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
      "(vert)" => quote! { rendiation_ral::ShaderStage::Vertex },
      "(frag)" => quote! { rendiation_ral::ShaderStage::Fragment },
      _ => panic!("unsupported"),
    };

    quote! { #field_name:<#ty as rendiation_shadergraph::ShaderGraphBindGroupItemProvider>::create_instance(#field_str, bindgroup_builder, #visibility), }
  })
  .collect();

  quote! {
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
  }
}

fn derive_wgpu_layout(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let struct_name = &input.ident;
  let fields = only_named_struct_fields(&input).unwrap();

  let wgpu_create_bindgroup_layout_create: Vec<_> = fields
    .iter()
    .map(|f| {
      let ty = &f.ty;
      let attr = f.attrs.iter().find(|a| a.path.is_ident("stage")).unwrap();
      let name = format!("{}", attr.tokens); // can i do better?
      let visibility = match name.as_str() {
        "(vert)" => quote! { rendiation_ral::ShaderStage::Vertex },
        "(frag)" => quote! { rendiation_ral::ShaderStage::Fragment },
        _ => panic!("unsupported"),
      };

      quote! {.bind(
       <#ty as rendiation_shadergraph::WGPUBindgroupItem>::to_layout_type(),
       #visibility
      )}
    })
    .collect();

  quote! {
    impl rendiation_webgpu::WGPUBindGroupLayoutProvider for #struct_name {

      fn provide_layout(renderer: &rendiation_webgpu::WGPURenderer) -> rendiation_webgpu::BindGroupLayout {
        rendiation_webgpu::BindGroupLayoutBuilder::new()
        #(#wgpu_create_bindgroup_layout_create)*
        .build(renderer)
      }
    }
  }
}
