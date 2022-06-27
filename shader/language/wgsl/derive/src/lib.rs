use proc_macro::TokenStream;
// use crate::shader::gen_meta_name;
use quote::{format_ident, quote};
use shader_derives_shared::*;
use wgsl_parser::*;

/// Create shadergraph function by parsing wgsl source code.
#[proc_macro]
pub fn wgsl_function(input: TokenStream) -> TokenStream {
  let input = format!("{}", input);
  gen_wgsl_function(input.as_str()).into()
}

fn gen_wgsl_function(wgsl: &str) -> proc_macro2::TokenStream {
  let fun = FunctionDefine::parse_input(wgsl).expect("wgsl parse error");

  let mut collector = ForeignImplCollector::default();
  fun.visit_by(&mut collector);
  let foreign_functions: Vec<_> = collector
    .depend_user_functions
    .iter()
    .map(|f_name| {
      let name = gen_fn_meta_name(f_name);
      quote! { #name, }
    })
    .collect();

  let foreign_types: Vec<_> = collector
    .depend_user_struct
    .iter()
    .map(|f_name| {
      let name = gen_struct_meta_name(f_name);
      quote! { #name, }
    })
    .collect();

  let function_name = fun.name.name.as_ref();
  let prototype_name = gen_fn_meta_name(function_name);
  let function_name = format_ident!("{}", function_name);
  let quoted_function_name = format!("{}", function_name);
  let quoted_source = wgsl.to_string();
  let function_source = quote! {#quoted_source};

  let return_type = fun
    .return_type
    .as_ref()
    .map(convert_type)
    .unwrap_or(quote! {()});

  let (gen_function_inputs, input_node_prepare): (Vec<_>, Vec<_>) = fun
    .arguments
    .iter()
    .map(|(name, ty)| {
      let name = format_ident!("{}", &name.name);
      let ty = convert_type(ty);
      (
        quote! { #name: impl Into<Node<#ty>>, },
        quote! {
         let #name = #name.into().handle();
         parameters.push(#name);
        },
      )
    })
    .unzip();

  quote! {
    #[allow(non_upper_case_globals)]
    pub static #prototype_name: shadergraph::ShaderFunctionMetaInfo =
      shadergraph::ShaderFunctionMetaInfo{
        function_name: #quoted_function_name,
        function_source: #function_source,
        depend_functions:&[
          #(#foreign_functions)*
        ],
        depend_types: &[
          #(#foreign_types)*
        ]
      };

    #[allow(clippy::too_many_arguments)]
    pub fn #function_name (
      #(#gen_function_inputs)*
    ) -> shadergraph::Node<#return_type> {
      use shadergraph::*;

      let mut parameters = Vec::new();
      #(#input_node_prepare)*

      ShaderGraphNodeExpr::FunctionCall {
        meta: & #prototype_name,
        parameters,
      }.insert_graph()
    }
  }
}

fn convert_scalar(ty: &PrimitiveValueType) -> proc_macro2::TokenStream {
  match ty {
    PrimitiveValueType::Float32 => quote! { f32 },
    PrimitiveValueType::UnsignedInt32 => quote! { u32 },
    PrimitiveValueType::Int32 => quote! { i32 },
    PrimitiveValueType::Bool => quote! { bool },
  }
}

fn convert_type(ty: &TypeExpression) -> proc_macro2::TokenStream {
  match ty {
    TypeExpression::Struct(name) => {
      let name = format_ident!("{}", &name.name);
      quote! { #name }
    }
    TypeExpression::Primitive(p) => match p {
      PrimitiveType::Scalar(sty) => convert_scalar(sty),
      PrimitiveType::Vector(PrimitiveVectorType {
        value_ty,
        vec_ty: data_ty,
      }) => {
        let inner = convert_scalar(value_ty);
        match data_ty {
          PrimitiveVecDataType::Vec2 => quote! { shadergraph::Vec2<#inner> },
          PrimitiveVecDataType::Vec3 => quote! { shadergraph::Vec3<#inner> },
          PrimitiveVecDataType::Vec4 => quote! { shadergraph::Vec4<#inner> },
          PrimitiveVecDataType::Mat2 => quote! { shadergraph::Mat2<#inner> },
          PrimitiveVecDataType::Mat3 => quote! { shadergraph::Mat3<#inner> },
          PrimitiveVecDataType::Mat4 => quote! { shadergraph::Mat4<#inner> },
        }
      }
      PrimitiveType::Texture(TextureType {
        value_ty,
        container_ty,
      }) => {
        let _ = convert_scalar(value_ty); // todo
        match container_ty {
          TextureContainerType::D1 => todo!(),
          TextureContainerType::D2 => quote! { shadergraph::ShaderTexture },
          TextureContainerType::D2Array => todo!(),
          TextureContainerType::D3 => todo!(),
          TextureContainerType::Cube => todo!(),
          TextureContainerType::CubeArray => todo!(),
        }
      }
      PrimitiveType::Sampler => quote! { shadergraph::ShaderSampler },
    },
  }
}