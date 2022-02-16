use crate::utils::StructInfo;
use quote::quote;

pub fn derive_vertex_impl(input: syn::DeriveInput) -> proc_macro2::TokenStream {
  let s = StructInfo::new(&input);

  let vertex_attributes: Vec<_> = s
    .fields_raw
    .iter()
    .map(|f| {
      let field_name = f.ident.as_ref().unwrap();
      let ty = &f.ty;

      let attr = f
        .attrs
        .iter()
        .find(|a| a.path.is_ident("semantic"))
        .unwrap();

      quote! {
        VertexAttribute {
          format: <#ty as VertexInShaderGraphNodeType>::to_vertex_format(),
          offset: offset_of!(Self, #field_name) as u64,
          shader_location: builder.register_vertex_in::<#attr.tokens>(),
        },
      }
    })
    .collect();

  quote! {
    impl shadergraph::ShaderGraphVertexInProvider for Vertex {
      fn provide_layout_and_vertex_in(
        builder: &mut shadergraph::ShaderGraphVertexBuilder,
        step_mode: shadergraph::VertexStepMode
      ) {
        use shadergraph::*;

        let layout = ShaderGraphVertexBufferLayout {
          array_stride: std::mem::size_of::<Self>() as u64,
          step_mode,
          attributes: vec![
            #(#vertex_attributes)*
          ],
        };
        builder.push_vertex_layout(layout);
      }
    }
  }
}
