use proc_macro::TokenStream;

use glsl::parser::Parse;
use glsl::syntax;
use quote::{format_ident, quote};
use syn::parse_macro_input;

#[proc_macro]
pub fn glsl(input: TokenStream) -> TokenStream {
  todo!()
}

#[proc_macro]
pub fn glsl_function(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::LitStr);
  let glsl = input.value();
  gen_glsl_function(&glsl).into()
}

fn gen_glsl_function(glsl: &str) -> proc_macro2::TokenStream {
  let glsl = glsl.trim_start();
  let parsed = syntax::FunctionDefinition::parse(glsl).unwrap();

  let function_name = parsed.prototype.name.as_str();

  let function_name = format_ident!("{}", function_name);
  let quoted_function_name = format!("\"{}\"", function_name);
  let quoted_source = format!("\"{}\"", glsl);

  // https://docs.rs/glsl/4.1.1/glsl/syntax/struct.FunctionPrototype.html
  let return_type = convert_type(&parsed.prototype.ty.ty.ty);

  // https://docs.rs/glsl/4.1.1/glsl/syntax/struct.FunctionParameterDeclarator.html
  let params: Vec<_> = parsed
    .prototype
    .parameters
    .iter()
    .map(|d| {
      if let syntax::FunctionParameterDeclaration::Named(_, p) = d {
        let ty = &p.ty;
        if ty.array_specifier.is_some() {
          panic!("unsupported") // todo improve
        }
        let name = p.ident.ident.as_str();
        (convert_type(&ty.ty), format_ident!("{}", name))
      } else {
        panic!("unsupported") // todo improve
      }
    })
    .collect();

  let gen_function_inputs: Vec<_> = params
    .iter()
    .map(|(ty, name)| {
      quote! { #name: ShaderGraphNodeHandle<#ty>, }
    })
    .collect();

  let gen_node_connect: Vec<_> = params
    .iter()
    .map(|(_, name)| {
      quote! { graph.nodes.connect_node(#name.cast_type(), result); }
    })
    .collect();

  quote! {
    use rendiation_math::*;
    use rendiation_shadergraph::*;

    pub fn #function_name (
      #(#gen_function_inputs)*
    ) -> ShaderGraphNodeHandle<#return_type> {
      let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
      let graph = guard.as_mut().unwrap();
      let result = graph
        .nodes
        .create_node(ShaderGraphNode::new(
            ShaderGraphNodeData::Function(
            FunctionNode {
              function_name: #quoted_function_name,
              function_source: #quoted_source,
            },
          )
        )
      );
      unsafe {
        #(#gen_node_connect)*
        result.cast_type()
      }
    }

  }
}

// fn uncharted2ToneMapping(
//   intensity: ShaderGraphNodeHandle<f32>,
//   toneMappingExposure: ShaderGraphNodeHandle<f32>,
//   toneMappingWhitePoint: ShaderGraphNodeHandle<f32>,
// ) -> ShaderGraphNodeHandle<f32> {
//   let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
//   let graph = guard.as_mut().unwrap();
//   let result = graph
//     .nodes
//     .create_node(ShaderGraphNode::new(ShaderGraphNodeData::Function(
//       FunctionNode {
//         function_name: "test",
//         function_source: "sdfsdfadsfadfsdf",
//       },
//     )));
//   unsafe {
//     graph.nodes.connect_node(intensity.cast_type(), result);
//     graph.nodes.connect_node(toneMappingExposure.cast_type(), result);
//     graph.nodes.connect_node(toneMappingWhitePoint.cast_type(), result);
//     result.cast_type()
//   }
// }

fn convert_type(glsl: &syntax::TypeSpecifierNonArray) -> proc_macro2::TokenStream {
  use syntax::TypeSpecifierNonArray::*;
  match glsl {
    Float => quote! { f32 },
    Vec2 => quote! { Vec2<f32> },
    Vec3 => quote! { Vec3<f32> },
    Vec4 => quote! { Vec4<f32> },
    _ => panic!("unsupported param type"),
  }
}
