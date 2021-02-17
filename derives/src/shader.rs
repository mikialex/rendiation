use crate::utils::StructInfo;
use quote::TokenStreamExt;
use quote::{format_ident, quote};

pub fn derive_shader_impl(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let s = StructInfo::new(input);
  let mut generated = proc_macro2::TokenStream::new();
  generated.append_all(derive_shadergraph_instance(&s));
  generated.append_all(derive_ral_resource_instance(&s));
  generated.append_all(derive_webgl_upload_instance(&s));
  generated
}

fn derive_webgl_upload_instance(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;
  let instance_name = format_ident!("{}WebGLUniformUploadInstance", struct_name);
  let ral_instance_name = format_ident!("{}RALResourceInstance", struct_name);

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
    quote! { self.#field_name.upload(resources.bindgroups.get_bindgroup_unwrap::<#ty>(value.#field_name), renderer, resources); }
  });

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
      ) -> Self {
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

    #[cfg(feature = "webgl")]
    impl rendiation_webgl::WebGLUniformUploadShaderInstanceBuilder for #ral_instance_name<rendiation_webgl::WebGL> {
      fn create_uploader(
        &self,
        gl: &rendiation_webgl::WebGl2RenderingContext,
        program: &rendiation_webgl::WebGlProgram,
      ) -> Box<dyn rendiation_webgl::WebGLUniformUploadShaderInstance>{
        Box::new(#instance_name::create("", gl, program))
      }
    }

  }
}

fn derive_shadergraph_instance(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;
  let shadergraph_instance_name = format_ident!("{}ShaderGraphShaderInstance", struct_name);

  let shadergraph_instance_fields = s.map_fields(|(field_name, ty)| {
    quote! { pub #field_name: <#ty as rendiation_shadergraph::ShaderGraphBindGroupProvider>::ShaderGraphBindGroupInstance, }
  });

  let instance_create = s.map_fields(|(field_name, ty)| {
    quote! { #field_name: builder.bindgroup_by::<#ty>(), }
  });

  quote! {
    pub struct #shadergraph_instance_name {
      #(#shadergraph_instance_fields)*
    }

    impl rendiation_shadergraph::ShaderGraphBuilderCreator for #struct_name {
      type ShaderGraphShaderInstance = #shadergraph_instance_name;

      fn create_builder() -> (
        rendiation_shadergraph::ShaderGraphBuilder,
        Self::ShaderGraphShaderInstance,
        <Self::ShaderGeometry as ShaderGraphGeometryProvider>::ShaderGraphGeometryInstance
      ) {
        let mut builder = rendiation_shadergraph::ShaderGraphBuilder::new();
        let instance = #shadergraph_instance_name {
          #(#instance_create)*
        };
        let geometry_instance = <Self::ShaderGeometry>::create_instance(&mut builder);
        (builder, instance, geometry_instance)
      }
    }

  }
}

fn derive_ral_resource_instance(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;
  let resource_instance_name = format_ident!("{}RALResourceInstance", struct_name);

  let resource_struct_fields = s.map_fields(|(field_name, ty)| {
    quote! { pub #field_name: rendiation_ral::BindGroupHandle<T, #ty>, }
  });

  let bindgroup_active_pass = s.map_fields_with_index(|(i, (field_name, _))| {
    quote! { resources.bindgroups.get_bindgroup_boxed(instance.#field_name).apply(render_pass, resources, #i, gpu_shading); }
  });

  let create_resource_instance_fn_param = s.map_fields(|(field_name, ty)| {
    quote! {#field_name: rendiation_ral::BindGroupHandle<T, #ty>,}
  });

  let create_resource_instance_field = s.map_fields(|(field_name, _ty)| {
    quote! {#field_name,}
  });

  quote! {
    #[derive(Clone)]
    pub struct #resource_instance_name<T: rendiation_ral::RAL> {
      #(#resource_struct_fields)*
    }

    impl<T: rendiation_ral::RAL> rendiation_ral::ShadingProvider<T> for #struct_name {
      type Instance = #resource_instance_name<T>;
      fn apply(
        instance: &Self::Instance,
        gpu_shading: &<T as rendiation_ral::RAL>::Shading,
        render_pass: &mut <T as rendiation_ral::RAL>::RenderPass,
        resources: &rendiation_ral::ResourceManager<T>,
      ) {
        let resources: &'static rendiation_ral::ResourceManager<T> = unsafe {std::mem::transmute(resources)};
        T::apply_shading(render_pass, gpu_shading);
        #(#bindgroup_active_pass)*
      }

    }

    // maybe we should impl T for this?
    #[cfg(feature = "webgpu")]
    impl rendiation_ral::ShadingCreator<rendiation_webgpu::WebGPU> for  #struct_name {
      fn create_shader(
        _instance: &Self::Instance,
        renderer: &mut <rendiation_webgpu::WebGPU as rendiation_ral::RAL>::Renderer
      ) -> <rendiation_webgpu::WebGPU as rendiation_ral::RAL>::Shading {
        let build_source = rendiation_webgpu::convert_build_source(&#struct_name::build_graph());
        rendiation_webgpu::WebGPU::create_shading(renderer, &build_source)
      }
    }

    #[cfg(feature = "webgl")]
    impl rendiation_ral::ShadingCreator<rendiation_webgl::WebGL> for  #struct_name {
      fn create_shader(
        instance: &#resource_instance_name<rendiation_webgl::WebGL>, // this must specify type, cant use Self::Instance, because the uploader_creator require trait impl
        renderer: &mut <rendiation_webgl::WebGL as rendiation_ral::RAL>::Renderer
      ) -><rendiation_webgl::WebGL as rendiation_ral::RAL>::Shading {
        let compiled = #struct_name::build_graph().compile();
        let instance_c = instance.clone();
        let build_source = rendiation_webgl::WebGLProgramBuildSource {
          glsl_vertex: compiled.vertex_shader.clone(),
          glsl_fragment: compiled.frag_shader.clone(),
          uploader_creator: Box::new(instance_c),
        };
        rendiation_webgl::WebGL::create_shading(renderer, &build_source)
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
