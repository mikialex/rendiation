use crate::utils::only_named_struct_fields;
use quote::TokenStreamExt;
use quote::{format_ident, quote};

pub fn derive_shader_impl(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let mut generated = proc_macro2::TokenStream::new();
  generated.append_all(derive_shadergraph_instance(input));
  generated.append_all(derive_ral_resource_instance(input));
  generated.append_all(derive_webgl_upload_instance(input));
  generated.append_all(derive_shader_nyxt_wasm_instance_impl(input));
  generated
}

fn derive_shader_nyxt_wasm_instance_impl(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let struct_name = &input.ident;
  let instance_name = format_ident!("{}WASM", struct_name);

  let fields = only_named_struct_fields(input).unwrap();
  let fields_info: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap().clone();
      let ty = f.ty.clone();
      (field_name, ty)
    })
    .collect();

  let fields_wasm_getter_setter: Vec<_> = fields_info
    .iter()
    .map(|(field_name, ty)| {
      let getter_name = format_ident!("get_{}", field_name);
      let setter_name = format_ident!("set_{}", field_name);
      quote! {
        #[wasm_bindgen(getter)]
        pub fn #getter_name(&self) -> <#ty as NyxtBindGroupWrapped>::Wrapper {
          self.inner.mutate_item(|d| d.#field_name.clone())
        }
        #[wasm_bindgen(setter)]
        pub fn #setter_name(&mut self, value: <#ty as NyxtBindGroupWrapped>::Wrapper) {
          self.inner.mutate_item(|d| d.#field_name = value.clone())
        }
      }
    })
    .collect();

  let constructor_parameters: Vec<_> = fields_info
    .iter()
    .map(|(field_name, ty)| {
      quote! {#field_name: &<#ty as NyxtBindGroupWrapped>::Wrapper,}
    })
    .collect();

  let constructor_create_ral_instance: Vec<_> = fields_info
    .iter()
    .map(|(field_name, _ty)| {
      quote! { #field_name.handle, }
    })
    .collect();

  quote! { 
    #[cfg(feature = "nyxt")]
    use wasm_bindgen::prelude::*;

    #[cfg(feature = "nyxt")]
    #[wasm_bindgen]
    pub struct #instance_name {
      inner: nyxt_core::NyxtViewerHandledObject<nyxt_core::ShadingHandleWrap<#struct_name>>,
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
          inner.resource.shadings.add(default_value)
        });
        Self {
          inner: viewer.make_handle_object(nyxt_core::UniformHandleWrap(handle)),
        }
      }
    }

  }
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
      quote! { 
        self.#field_name.upload(resources.bindgroups.get_bindgroup_unwrap::<#ty>(value.#field_name), renderer, resources);
       }
    })
    .collect();

  let ral_instance_name = format_ident!("{}RALResourceInstance", struct_name);

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
      type UploadValue = <#struct_name as rendiation_ral::ShadingProvider<rendiation_webgl::WebGL>>::Instance;
      type UploadInstance = #instance_name;
    }

    #[cfg(feature = "webgl")]
    use rendiation_webgl::UploadInstance;
    #[cfg(feature = "webgl")]
    impl rendiation_webgl::WebGLUniformUploadShaderInstance for #instance_name {
      fn upload_all(
        &mut self,
        renderer: &mut rendiation_webgl::WebGLRenderer,
        resource_manager: &rendiation_ral::ResourceManager<rendiation_webgl::WebGL>,
        handle_object: &dyn std::any::Any,
      ){
        self.upload(handle_object.downcast_ref::<&#ral_instance_name<rendiation_webgl::WebGL>>().unwrap(), renderer, resource_manager)
      }
    }

  }
}

fn derive_shadergraph_instance(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let struct_name = &input.ident;
  let shadergraph_instance_name = format_ident!("{}ShaderGraphShaderInstance", struct_name);
  let fields = only_named_struct_fields(&input).unwrap();  
  let fields_info: Vec<_> = fields
  .iter()
  .map(|f| {
    let field_name = f.ident.as_ref().unwrap().clone();
    let ty = f.ty.clone();
    (field_name, ty)
  })
  .collect();

  let shadergraph_instance_fields: Vec<_> = fields_info
  .iter()
  .map(|(field_name, ty)| {
    quote! { pub #field_name: <#ty as rendiation_shadergraph::ShaderGraphBindGroupProvider>::ShaderGraphBindGroupInstance, }
  })
  .collect();

  let instance_create: Vec<_> = fields_info
  .iter()
  .map(|(field_name, ty)| {
      quote! { #field_name: builder.bindgroup_by::<#ty>(), }
    })
    .collect();

  quote! {
    pub struct #shadergraph_instance_name {
      #(#shadergraph_instance_fields)*
    }

    impl rendiation_shadergraph::ShaderGraphBuilderCreator for #struct_name {
      type ShaderGraphShaderInstance = #shadergraph_instance_name;

      fn create_builder(
      ) -> (rendiation_shadergraph::ShaderGraphBuilder, Self::ShaderGraphShaderInstance) {
        let mut builder = rendiation_shadergraph::ShaderGraphBuilder::new();
        let instance = #shadergraph_instance_name {
          #(#instance_create)*
        };
        (builder, instance)
      }
    }

  }
}

fn derive_ral_resource_instance(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let struct_name = &input.ident;
  let resource_instance_name = format_ident!("{}RALResourceInstance", struct_name);
  let fields = only_named_struct_fields(&input).unwrap();

  let fields_info: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap().clone();
      let ty = f.ty.clone();
      (field_name, ty)
    })
    .collect();
  

  let resource_struct_fields: Vec<_> = fields_info
    .iter()
    .map(|(field_name, ty)| {
      quote! { pub #field_name: rendiation_ral::BindGroupHandle<T, #ty>, }
    })
    .collect();

  let bindgroup_active_pass: Vec<_> = fields_info
    .iter()
    .enumerate()
    .map(|(i, (field_name, _))| {
      quote! { resources.bindgroups.get_bindgroup_boxed(instance.#field_name).apply(render_pass, resources, #i, gpu_shading); }
    })
    .collect();

  let create_resource_instance_fn_param: Vec<_> = fields_info
    .iter()
    .map(|(field_name, ty)| {
      quote! {#field_name: rendiation_ral::BindGroupHandle<T, #ty>,}
    })
    .collect();

  let create_resource_instance_field: Vec<_> = fields_info
    .iter()
    .map(|(field_name, _ty)| {
      quote! {#field_name,}
    })
    .collect();

  quote! {

    #[cfg_attr(feature = "nyxt", wasm_bindgen)]
    pub struct #resource_instance_name<T: rendiation_ral::RAL> {
      #(#resource_struct_fields)*
    }

    impl<T: rendiation_ral::RAL> rendiation_ral::ShadingProvider<T> for #struct_name {
      type Instance = #resource_instance_name<T>;
      fn apply(
        instance: &Self::Instance,
        gpu_shading: &T::Shading,
        render_pass: &mut T::RenderPass,
        resources: &rendiation_ral::ResourceManager<T>,
      ) {
        let resources: &'static rendiation_ral::ResourceManager<T> = unsafe {std::mem::transmute(resources)};
        T::apply_shading(render_pass, gpu_shading);
        #(#bindgroup_active_pass)*
      }
    }

    impl #struct_name {
      pub fn create_resource_instance<T: rendiation_ral::RAL>(
        #(#create_resource_instance_fn_param)*
      ) -> #resource_instance_name<T> {
        #resource_instance_name {
          #(#create_resource_instance_field)*
        }
      }
    }
  }
}
