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
        .find(|a| a.path().is_ident("semantic"))
        .expect("require semantic attribute");
      let token = attr.parse_args::<syn::Type>().expect("expect type");

      quote! {
       < #ty as VertexInBuilder >::build_attribute::<#token>(&mut list_builder, builder);
      }
    })
    .collect();

  // The attributes are tightly packed in the field order, so the rust layout must be compact,
  // which is checked at compile time. The generic struct is not checked, because the const item
  // can not use the generic parameters.
  let layout_check = input.generics.params.is_empty().then(|| {
    let field_checks = s.fields_info.iter().map(|(name, ty)| {
      quote! {
        assert!(
          ::core::mem::offset_of!(#struct_name, #name) == offset,
          concat!(
            "the vertex struct field `",
            stringify!(#name),
            "` is not tightly packed after the previous field, the rust layout has padding"
          )
        );
        offset += ::core::mem::size_of::<#ty>();
      }
    });
    quote! {
      const _: () = {
        let mut offset = 0;
        #(#field_checks)*
        assert!(
          ::core::mem::size_of::<#struct_name>() == offset,
          "the vertex struct has trailing padding"
        );
      };
    }
  });

  quote! {
    #layout_check

    impl rendiation_shader_api::ShaderVertexInProvider for #struct_name {
      fn provide_layout_and_vertex_in(
        builder: &mut rendiation_shader_api::ShaderRawVertexBuilder,
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
