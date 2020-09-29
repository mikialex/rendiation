use crate::utils::only_named_struct_fields;
use quote::TokenStreamExt;
use quote::{format_ident, quote};

pub fn derive_shader_impl(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let mut generated = proc_macro2::TokenStream::new();
  generated.append_all(derive_shadergraph_instance(input));
  generated.append_all(derive_ral_resource_instance_wgpu(input));
  generated
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

fn derive_ral_resource_instance_wgpu(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let struct_name = &input.ident;
  let resource_instance_name = format_ident!("{}RALResourceInstance_WGPU", struct_name);
  let fields = only_named_struct_fields(&input).unwrap();

  let resource_struct_fields: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      let ty = &f.ty;
      quote! { pub #field_name: rendiation_ral::BindGroupHandle<WGPURenderer, #ty>, }
    })
    .collect();

  let bindgroup_active_pass: Vec<_> = fields
    .iter()
    .enumerate()
    .map(|(i, f)| {
      let field_name = f.ident.as_ref().unwrap();
      quote! { render_pass.set_bindgroup(#i, resources.get_gpu(instance.#field_name)); }
    })
    .collect();

  let create_resource_instance_fn_param: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      let ty = &f.ty;
      quote! {#field_name: rendiation_ral::BindGroupHandle<WGPURenderer, #ty>,}
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
    pub struct #resource_instance_name {
      #(#resource_struct_fields)*
    }

    impl rendiation_ral::ShadingProvider<WGPURenderer> for #struct_name {
      type Instance = #resource_instance_name;
      fn apply(
        instance: &Self::Instance,
        gpu_shading: &<WGPURenderer as rendiation_ral::RALBackend>::Shading,
        render_pass: &mut <WGPURenderer as rendiation_ral::RALBackend>::RenderPass,
        resources: &rendiation_ral::BindGroupManager<WGPURenderer>,
      ) {
        // render_pass is cast to static, so resources must cast to static too..
        let resources: &'static rendiation_ral::BindGroupManager<WGPURenderer> = unsafe {std::mem::transmute(resources)};
        let gpu: &'static WGPUPipeline = unsafe {std::mem::transmute(gpu_shading)};
        render_pass.set_pipeline(gpu);
        #(#bindgroup_active_pass)*
      }
    }

    impl #struct_name {
      pub fn create_resource_instance(
        #(#create_resource_instance_fn_param)*
      ) ->  #resource_instance_name {
        #resource_instance_name {
          #(#create_resource_instance_field)*
        }
      }
    }
  }
}

// fn derive_ral_resource_instance_webgl(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
//   let struct_name = &input.ident;
//   let resource_instance_name = format_ident!("{}RALResourceInstance_WebGL", struct_name);

//   quote! {
//     pub struct #resource_instance_name {
//       #(#resource_struct_fields)*
//     }

//     impl rendiation_ral::ShadingProvider<WebGLRenderer> for #struct_name {
//       type Instance = #resource_instance_name;
//       fn apply(
//         instance: &rendiation_ral::ShadingPair<WebGLRenderer, Self>,
//         render_pass: &mut <WebGLRenderer as rendiation_ral::RALBackend>::RenderPass,
//         gpu_shading: &<WebGLRenderer as rendiation_ral::RALBackend>::Shading,
//         resources: &rendiation_ral::BindGroupManager<WebGLRenderer>,
//       ) {
//         let handle_instance = &instance.data;
//         render_pass.use_program(Some(gpu_shading));
//         #(#bindgroup_active_pass)*
//       }
//     }

//     impl #struct_name {
//       pub fn create_resource_instance(
//         #(#create_resource_instance_fn_param)*
//       ) ->  #resource_instance_name {
//         #resource_instance_name {
//           #(#create_resource_instance_field)*
//         }
//       }
//     }
//   }
// }
