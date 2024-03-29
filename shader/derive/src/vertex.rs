use quote::quote;

use crate::utils::StructInfo;

pub fn derive_vertex_impl(input: syn::DeriveInput) -> proc_macro2::TokenStream {
  let s = StructInfo::new(&input);
  let struct_name = &s.struct_name;

  let vertex_attributes: Vec<_> = s
    .fields_raw
    .iter()
    .map(|f| {
      let ty = &f.ty;

      let attr = f
        .attrs
        .iter()
        .find(|a| a.path.is_ident("semantic"))
        .expect("require semantic attribute");
      let token = attr.parse_args::<syn::Type>().expect("expect type");

      quote! {
       < #ty as VertexInBuilder >::build_attribute::<#token>(&mut list_builder, builder);
      }
    })
    .collect();

  quote! {
    impl rendiation_shader_api::ShaderVertexInProvider for #struct_name {
      fn provide_layout_and_vertex_in(
        builder: &mut rendiation_shader_api::ShaderVertexBuilder,
        step_mode: rendiation_shader_api::VertexStepMode
      ) {
        use rendiation_shader_api::*;

        let mut list_builder = AttributesListBuilder::default();
        #(#vertex_attributes)*
        list_builder.build(builder, step_mode);
      }
    }
  }
}
