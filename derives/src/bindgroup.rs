extern crate proc_macro;

use crate::utils::StructInfo;
use quote::TokenStreamExt;
use quote::{format_ident, quote};

pub fn derive_bindgroup_impl(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let s = StructInfo::new(input);
  let mut generated = proc_macro2::TokenStream::new();
  generated.append_all(derive_ral_bindgroup_layout(&s));
  generated.append_all(derive_shadergraph_instance(&s));
  generated.append_all(derive_ral_bindgroup(&s));
  generated.append_all(derive_webgl_upload_instance(&s));
  generated.append_all(derive_bindgroup_nyxt_wasm_instance_impl(&s));
  generated
}

fn derive_bindgroup_nyxt_wasm_instance_impl(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;
  let instance_name = format_ident!("{}WASM", struct_name);

  let fields_wasm_getter_setter: Vec<_> = s.map_fields(|(field_name, ty)| {
    let getter_name = format_ident!("get_{}", field_name);
    let setter_name = format_ident!("set_{}", field_name);
    quote! {
      pub fn #getter_name(&self) -> <#ty as NyxtUBOWrapped>::Wrapper {
        let mut viewer = self.inner.clone_viewer();
        self.inner.mutate_item(|d| #ty::to_nyxt_wrapper(&mut viewer, d.#field_name.clone()))
      }
      pub fn #setter_name(&mut self, value: <#ty as NyxtUBOWrapped>::Wrapper) {
        self.inner.mutate_item(|d| d.#field_name = value.inner.handle.0);
      }
    }
  });

  let constructor_parameters = s.map_fields(|(field_name, ty)| {
    quote! {#field_name: &<#ty as NyxtUBOWrapped>::Wrapper,}
  });

  let constructor_create_ral_instance = s.map_fields(|(field_name, _ty)| {
    quote! { #field_name.inner.handle.0, }
  });

  quote! {
    #[cfg(feature = "nyxt")]
    use wasm_bindgen::prelude::*;

    #[cfg(feature = "nyxt")]
    #[wasm_bindgen]
    pub struct #instance_name {
      #[wasm_bindgen(skip)]
      pub inner: nyxt_core::NyxtViewerHandledObject<nyxt_core::BindGroupHandleWrap<#struct_name>>,
    }

    #[cfg(feature = "nyxt")]
    impl nyxt_core::NyxtBindGroupWrapped for #struct_name {
      type Wrapper = #instance_name;
      fn to_nyxt_wrapper(viewer: &mut NyxtViewer, handle: rendiation_ral::BindGroupHandle<GFX, Self>) -> Self::Wrapper{
        #instance_name {
          inner: viewer.make_handle_object(nyxt_core::BindGroupHandleWrap(handle)),
        }
      }
    }

    #[cfg(feature = "nyxt")]
    #[wasm_bindgen]
    impl #instance_name {
      #(#fields_wasm_getter_setter)*

      #[wasm_bindgen(constructor)]
      pub fn new(viewer: &mut nyxt_core::NyxtViewer,
        #(#constructor_parameters)*
      ) -> Self {
        let handle = viewer.mutate_inner(|inner| {
          let default_value = #struct_name::create_resource_instance(
            #(#constructor_create_ral_instance)*
          );
          inner.resource.add_bindgroup(default_value)
        });
        use nyxt_core::NyxtBindGroupWrapped;
        #struct_name::to_nyxt_wrapper(viewer, handle)
      }
    }

  }
}

fn derive_webgl_upload_instance(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;
  let instance_name = format_ident!("{}WebGLUniformUploadInstance", struct_name);

  let instance_fields = s.map_fields(|(field_name, ty)| {
    quote! { pub #field_name: <#ty as rendiation_webgl::WebGLUniformUploadable>::UploadInstance, }
  });

  let instance_create = s.map_fields(|(field_name, ty)| {
    let field_str = format!("{}", field_name);
      quote! { #field_name:
       < <#ty as rendiation_webgl::WebGLUniformUploadable>::UploadInstance
       as rendiation_webgl::UploadInstance<#ty> >::create(
          format!("{}", #field_str).as_str(),
          gl,
          program
       ),
      }
  });

  let instance_upload = s.map_fields(|(field_name, ty)| { 
    quote! { self.#field_name.upload(unsafe{std::mem::transmute(&<#ty as rendiation_ral::RALBindgroupItem<rendiation_webgl::WebGL>>::get_item(value.#field_name, &resources.bindable))}, renderer, resources); }
  });

  let ral_instance_name = format_ident!("{}RALInstance", struct_name);

  quote! {

    #[cfg(feature = "webgl")]
    pub struct #instance_name {
      #(#instance_fields)*
    }

    #[cfg(feature = "webgl")]
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
        value: &#ral_instance_name<rendiation_webgl::WebGL>,
        renderer: &mut rendiation_webgl::WebGLRenderer,
        resources: &rendiation_ral::ResourceManager<rendiation_webgl::WebGL>,
      ){
        #(#instance_upload)*
      }
    }

    #[cfg(feature = "webgl")]
    impl rendiation_webgl::WebGLUniformUploadable for #struct_name {
      type UploadValue = <#struct_name as rendiation_ral::BindGroupProvider<rendiation_webgl::WebGL>>::Instance;
      type UploadInstance = #instance_name;
    }
  }
}

fn derive_ral_bindgroup(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;
  let ral_instance_name = format_ident!("{}RALInstance", struct_name);

  let ral_fields = s.map_fields(|(field_name, ty)| {
    quote! { pub #field_name: < #ty as rendiation_ral::RALBindgroupHandle<T>>::HandleType, }
  });

  let wgpu_resource_get = s.map_fields(|(field_name, ty)| {
    quote! {let #field_name = <#ty as rendiation_ral::RALBindgroupItem<rendiation_webgpu::WebGPU>>::get_item(instance.#field_name, resources); }
  });

  let create_resource_instance_fn_param = s.map_fields(|(field_name, ty)| {
    quote! {#field_name: < #ty as rendiation_ral::RALBindgroupHandle<T>>::HandleType, }
  });

  let create_resource_instance_field = s.map_fields(|(field_name, _)| {
    quote! {#field_name,}
  });

  let link = s.map_fields(|(_field_name, ty)| {
    quote! { #ty::add_reference(bindgroup_handle, resources); }
  });
  let unlink = s.map_fields(|(_field_name, ty)| {
    quote! { #ty::remove_reference(bindgroup_handle, resources); }
  });


  let wgpu_create_bindgroup_create = s.map_fields(|(field_name, ty)| {
    quote! {.push(<#ty as rendiation_webgpu::WGPUBindgroupItem>::to_binding(#field_name))}
  });

  quote! {
    pub struct #ral_instance_name<T: rendiation_ral::RAL> {
      #(#ral_fields)*
    }

    #[cfg(feature = "webgpu")]
    impl rendiation_ral::BindGroupCreator<rendiation_webgpu::WebGPU> for #struct_name {
      fn create_bindgroup(
        instance: &Self::Instance,
        renderer: &<rendiation_webgpu::WebGPU as rendiation_ral::RAL>::Renderer,
        resources: &rendiation_ral::ShaderBindableResourceManager<rendiation_webgpu::WebGPU>,
      ) -> <rendiation_webgpu::WebGPU as rendiation_ral::RAL>::BindGroup {
         #(#wgpu_resource_get)*

        rendiation_webgpu::BindGroupBuilder::new()
          #(#wgpu_create_bindgroup_create)*
          .build(
            &renderer.device,
            &renderer.bindgroup_layout_cache.get_bindgroup_layout_by_type::<#struct_name>(&renderer.device)
          )
      }
    }

    #[cfg(feature = "webgl")]
    impl rendiation_ral::BindGroupCreator<rendiation_webgl::WebGL> for #struct_name {
      fn create_bindgroup(
        instance: &Self::Instance,
        renderer: &<rendiation_webgl::WebGL as rendiation_ral::RAL>::Renderer,
        resources: &rendiation_ral::ShaderBindableResourceManager<rendiation_webgl::WebGL>,
      ) -> <rendiation_webgl::WebGL as rendiation_ral::RAL>::BindGroup {
        ()
      }
    }

    impl<T: rendiation_ral::RAL> rendiation_ral::BindGroupProvider<T> for #struct_name {
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

      fn add_reference(
        instance: &Self::Instance,
        bindgroup_handle: rendiation_ral::BindGroupHandle<T, rendiation_ral::AnyBindGroupType>,
        resources: &mut rendiation_ral::ShaderBindableResourceManager<T>,
      ){
        use rendiation_ral::RALBindgroupItem;
        #(#link)*
      }
      fn remove_reference(
        instance: &Self::Instance,
        bindgroup_handle: rendiation_ral::BindGroupHandle<T, rendiation_ral::AnyBindGroupType>,
        resources: &mut rendiation_ral::ShaderBindableResourceManager<T>,
      ){
        use rendiation_ral::RALBindgroupItem;
        #(#unlink)*
      }
    }

    impl #struct_name {
      pub fn create_resource_instance<T: rendiation_ral::RAL>(
        #(#create_resource_instance_fn_param)*
      ) ->  #ral_instance_name<T> {
        #ral_instance_name {
          #(#create_resource_instance_field)*
        }
      }
    }

  }
}

fn derive_shadergraph_instance(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;
  let shadergraph_instance_name = format_ident!("{}ShaderGraphInstance", struct_name);

  let instance_fields = s.map_fields(|(field_name, ty)| {
    quote! { pub #field_name: < #ty as rendiation_shadergraph::ShaderGraphBindGroupItemProvider>::ShaderGraphBindGroupItemInstance, }
  });

  let instance_new: Vec<_> = s.fields_raw.iter()
  .map(|f| {
    let field_name = f.ident.as_ref().unwrap();
    let ty = &f.ty;
    let field_str = format!("{}", field_name);
    let attr = f.attrs.iter().find(|a| a.path.is_ident("stage")).unwrap();
    let name = format!("{}", attr.tokens); // can i do better?
    let visibility = match name.as_str() {
      "(vert)" => quote! { rendiation_ral::ShaderStage::VERTEX },
      "(frag)" => quote! { rendiation_ral::ShaderStage::FRAGMENT },
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

fn derive_ral_bindgroup_layout(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;

  let wgpu_create_bindgroup_layout_create: Vec<_> = s.fields_raw.iter()
    .map(|f| {
      let ty = &f.ty;
      let attr = f.attrs.iter().find(|a| a.path.is_ident("stage")).unwrap();
      let name = format!("{}", attr.tokens); // can i do better?
      let visibility = match name.as_str() {
        "(vert)" => quote! { rendiation_ral::ShaderStage::VERTEX },
        "(frag)" => quote! { rendiation_ral::ShaderStage::FRAGMENT },
        _ => panic!("unsupported"),
      };

      quote! {.bind::<#ty>( #visibility)}
    })
    .collect();

  quote! {
    impl rendiation_ral::BindGroupLayoutDescriptorProvider for #struct_name {

      fn create_descriptor() -> Vec<rendiation_ral::BindGroupLayoutEntry> {
        rendiation_ral::BindGroupLayoutBuilder::new()
        #(#wgpu_create_bindgroup_layout_create)*
        .build()
      }
    }
  }
}
