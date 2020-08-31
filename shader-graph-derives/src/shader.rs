use crate::utils::only_named_struct_fields;
use quote::TokenStreamExt;
use quote::{format_ident, quote};

pub fn derive_shader_impl(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let mut generated = proc_macro2::TokenStream::new();
  generated.append_all(derive_shadergraph_instance(input));
  generated.append_all(derive_ral_resource_instance(input));
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
    struct #shadergraph_instance_name {
      #(#shadergraph_instance_fields)*
    }

    impl rendiation_shadergraph::ShaderGraphFactory<WGPURenderer> for #struct_name {
      type ShaderGraphShaderInstance = #shadergraph_instance_name;

      fn create_builder(
        renderer: &WGPURenderer,
      ) -> (rendiation_shadergraph::ShaderGraphBuilder, Self::ShaderGraphShaderInstance) {
        let builder = rendiation_shadergraph::ShaderGraphBuilder::new();
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
      quote! { render_pass.set_bindgroup(#i, resources.get_gpu(self.#field_name)); }
    })
    .collect();

  quote! {
    struct #resource_instance_name<T: rendiation_ral::RALBackend> {
      #(#resource_struct_fields)*
    }

    impl rendiation_ral::ShadingProvider<WGPURenderer> for #resource_instance_name<WGPURenderer> {
      fn apply(
        &self,
        render_pass: &mut <WGPURenderer as rendiation_ral::RALBackend>::RenderPass,
        gpu_shading: &<WGPURenderer as rendiation_ral::RALBackend>::Shading,
        resources: &rendiation_ral::BindGroupManager<WGPURenderer>,
      ) {
        #(#bindgroup_active_pass)*
      }
    }
  }
}
