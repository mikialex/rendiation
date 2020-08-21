use crate::utils::only_named_struct_fields;
use quote::{format_ident, quote};

pub fn derive_ubo_impl(input: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
  let struct_name = &input.ident;
  let shadergraph_instance_name = format_ident!("{}ShaderGraphInstance", struct_name);

  let struct_name_str = format!("{}", struct_name);
  let ubo_info_name = format_ident!("{}_UBO_INFO", struct_name);
  let fields = only_named_struct_fields(input)?;

  let ubo_info_gen: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      let field_str = format!("{}", field_name);
      let ty = &f.ty;
      quote! { .add_field::<#ty>(#field_str) }
    })
    .collect();

  let instance_fields: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      let ty = &f.ty;
      quote! { pub #field_name: rendiation_shadergraph::ShaderGraphNodeHandle< #ty >, }
    })
    .collect();

  let instance_new: Vec<_> = fields
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      let ty = &f.ty;
      let field_str = format!("{}", field_name);
      quote! { #field_name: ubo_builder.uniform::<#ty>(#field_str), }
    })
    .collect();

  let result = quote! {

    #[allow(non_upper_case_globals)]
    pub static #ubo_info_name: once_cell::sync::Lazy<
    std::sync::Arc<
      rendiation_shadergraph::UBOInfo
    >> =
    once_cell::sync::Lazy::new(||{
      std::sync::Arc::new(
        rendiation_shadergraph::UBOInfo::new(
          #struct_name_str,
        )
        #(#ubo_info_gen)*
        .gen_code_cache()
      )
    });

    pub struct #shadergraph_instance_name {
      #(#instance_fields)*
    }

    impl rendiation_shadergraph::ShaderGraphBindGroupItemProvider for #struct_name {
      type ShaderGraphBindGroupItemInstance = #shadergraph_instance_name;
      fn create_instance<'a>(
        name: &'static str, // uniform buffer group not need set name
        bindgroup_builder: &mut rendiation_shadergraph::ShaderGraphBindGroupBuilder<'a>,
        stage: rendiation_shadergraph::ShaderStage)
       -> Self::ShaderGraphBindGroupItemInstance {

        let mut ubo_builder = rendiation_shadergraph::UBOBuilder::new(
          #ubo_info_name.clone(),
          bindgroup_builder
        );

        let instance = Self::ShaderGraphBindGroupItemInstance {
          #(#instance_new)*
        };

        ubo_builder.ok(stage);
        instance
      }
    }

    impl rendiation_shadergraph::ShaderGraphUBO for #struct_name {}

  };

  Ok(result)
}
