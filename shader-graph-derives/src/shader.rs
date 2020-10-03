use crate::utils::only_named_struct_fields;
use quote::TokenStreamExt;
use quote::{format_ident, quote};

pub fn derive_shader_impl(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let mut generated = proc_macro2::TokenStream::new();
  generated.append_all(derive_shadergraph_instance(input));
  generated.append_all(derive_ral_resource_instance(input));
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
      quote! { 
        self.#field_name.upload(resources.bindgroups.get_bindgroup_unwrap::<#ty>(value.#field_name), renderer, resources);
       }
    })
    .collect();

  let ral_instance_name = format_ident!("{}RALResourceInstance", struct_name);

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
        renderer: &rendiation_webgl::WebGLRenderer,
        resources: &rendiation_ral::ResourceManager<rendiation_webgl::WebGLRenderer>,
      ){
        #(#instance_upload)*
      }
    }

    impl rendiation_webgl::WebGLUniformUploadable for #struct_name {
      type UploadValue = <#struct_name as rendiation_ral::ShadingProvider<rendiation_webgl::WebGLRenderer>>::Instance;
      type UploadInstance = #instance_name;
    }
  }
}

fn derive_shadergraph_instance(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let struct_name = &input.ident;
  let shadergraph_instance_name = format_ident!("{}ShaderGraphShaderInstance", struct_name);
  let fields = only_named_struct_fields(&input).unwrap();

  let shadergraph_instance_fields: Vec<_> = fields
  .iter()
  .map(|f| {
    let field_name = f.ident.as_ref().unwrap();
    let ty = &f.ty;
    quote! { pub #field_name: <#ty as rendiation_shadergraph::ShaderGraphBindGroupProvider>::ShaderGraphBindGroupInstance, }
  })
  .collect();

  let instance_create: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      let ty = &f.ty;
      quote! { #field_name: builder.bindgroup_by::<#ty>(renderer), }
    })
    .collect();

  quote! {
    pub struct #shadergraph_instance_name {
      #(#shadergraph_instance_fields)*
    }

    impl rendiation_shadergraph::ShaderGraphFactory<WGPURenderer> for #struct_name {
      type ShaderGraphShaderInstance = #shadergraph_instance_name;

      fn create_builder(
        renderer: &WGPURenderer,
      ) -> (rendiation_shadergraph::ShaderGraphBuilder, Self::ShaderGraphShaderInstance) {
        let mut builder = rendiation_shadergraph::ShaderGraphBuilder::new();
        let instance = BlockShaderShaderGraphShaderInstance {
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

  let resource_struct_fields: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      let ty = &f.ty;
      quote! { pub #field_name: rendiation_ral::BindGroupHandle<T, #ty>, }
    })
    .collect();

  let bindgroup_active_pass: Vec<_> = fields
    .iter()
    .enumerate()
    .map(|(i, f)| {
      let field_name = f.ident.as_ref().unwrap();
      quote! { resources.bindgroups.get_bindgroup_boxed(instance.#field_name).apply(render_pass, resources, #i, gpu_shading); }
    })
    .collect();

  let create_resource_instance_fn_param: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      let ty = &f.ty;
      quote! {#field_name: rendiation_ral::BindGroupHandle<T, #ty>,}
    })
    .collect();

  let create_resource_instance_field: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      quote! {#field_name,}
    })
    .collect();

  quote! {
    pub struct #resource_instance_name<T: rendiation_ral::RALBackend> {
      #(#resource_struct_fields)*
    }

    impl<T: rendiation_ral::RALBackend> rendiation_ral::ShadingProvider<T> for #struct_name {
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
      pub fn create_resource_instance<T: rendiation_ral::RALBackend>(
        #(#create_resource_instance_fn_param)*
      ) -> #resource_instance_name<T> {
        #resource_instance_name {
          #(#create_resource_instance_field)*
        }
      }
    }
  }
}
