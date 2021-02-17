use crate::utils::StructInfo;
use quote::TokenStreamExt;
use quote::{format_ident, quote};

pub fn derive_ubo_impl(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let s = StructInfo::new(input);
  let mut generated = proc_macro2::TokenStream::new();
  generated.append_all(derive_ubo_shadergraph_instance(&s));
  generated.append_all(derive_ubo_webgl_upload_instance(&s));
  generated
}

pub fn derive_ubo_webgl_upload_instance(s: &StructInfo) -> proc_macro2::TokenStream {
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
    quote! { <#ty as rendiation_webgl::WebGLUniformUploadable>::upload(&value.data.#field_name, &mut self.#field_name, renderer, resources); }
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
        program: &rendiation_webgl::WebGlProgram
      ) -> Self{
        Self {
          #(#instance_create)*
        }
      }
      fn upload(
        &mut self,
        value: &rendiation_ral::UniformBufferRef<'static, rendiation_webgl::WebGL, #struct_name>,
        renderer: &mut rendiation_webgl::WebGLRenderer,
        resources: &rendiation_ral::ResourceManager<rendiation_webgl::WebGL>,
      ){
        #(#instance_upload)*
      }
    }

    #[cfg(feature = "webgl")]
    impl rendiation_webgl::WebGLUniformUploadable for #struct_name {
      type UploadValue = rendiation_ral::UniformBufferRef<'static, rendiation_webgl::WebGL, #struct_name>;
      type UploadInstance = #instance_name;
    }
  }
}

pub fn derive_ubo_shadergraph_instance(s: &StructInfo) -> proc_macro2::TokenStream {
  let struct_name = &s.struct_name;
  let shadergraph_instance_name = format_ident!("{}ShaderGraphInstance", struct_name);

  let struct_name_str = format!("{}", struct_name);
  let ubo_info_name = format_ident!("{}_UBO_INFO", struct_name);

  let ubo_info_gen = s.map_fields(|(field_name, ty)| {
    let field_str = format!("{}", field_name);
    quote! { .add_field::<#ty>(#field_str) }
  });

  let instance_fields = s.map_fields(|(field_name, ty)| {
    quote! { pub #field_name: rendiation_shadergraph::Node<#ty>, }
  });

  let instance_new = s.map_fields(|(field_name, ty)| {
    let field_str = format!("{}", field_name);
    quote! { #field_name: ubo_builder.uniform::<#ty>(#field_str), }
  });

  quote! {
    #[allow(non_upper_case_globals)]
    pub static #ubo_info_name: once_cell::sync::Lazy<rendiation_shadergraph::UBOMetaInfo> =
    once_cell::sync::Lazy::new(|| {
        rendiation_shadergraph::UBOMetaInfo::new(
          #struct_name_str,
        )
        #(#ubo_info_gen)*
        .gen_code_cache()
    });

    pub struct #shadergraph_instance_name {
      #(#instance_fields)*
    }

    impl rendiation_shadergraph::ShaderGraphBindGroupItemProvider for #struct_name {
      type ShaderGraphBindGroupItemInstance = #shadergraph_instance_name;
      fn create_instance<'a>(
        name: &'static str, // uniform buffer group not need set name
        bindgroup_builder: &mut rendiation_shadergraph::ShaderGraphBindGroupBuilder<'a>,
        stage: rendiation_ral::ShaderStage)
       -> Self::ShaderGraphBindGroupItemInstance {

        let mut ubo_builder = rendiation_shadergraph::UBOBuilder::new(
          &#ubo_info_name,
          bindgroup_builder
        );

        let instance = Self::ShaderGraphBindGroupItemInstance {
          #(#instance_new)*
        };

        ubo_builder.ok(stage);
        instance
      }
    }

    impl rendiation_ral::UBOData for #struct_name {}

    #[cfg(feature = "webgpu")]
    impl rendiation_webgpu::WGPUUBOData for #struct_name {}
  }
}
