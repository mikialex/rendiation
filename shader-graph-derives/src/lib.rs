use proc_macro::TokenStream;

use glsl::parser::Parse;
use glsl::syntax;
use quote::{format_ident, quote};
use syn::parse_macro_input;

#[proc_macro]
pub fn glsl(_input: TokenStream) -> TokenStream {
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

  let prototype_name = format_ident!("{}_FUNCTION", function_name);
  let function_name = format_ident!("{}", function_name);
  let quoted_function_name = format!("\"{}\"", function_name);
  let quoted_source = format!("\"{}\"", glsl);

  // https://docs.rs/glsl/4.1.1/glsl/syntax/struct.FunctionPrototype.html
  let return_type = convert_type(&parsed.prototype.ty.ty.ty);
  let function_node_type = convert_node_type(&parsed.prototype.ty.ty.ty);

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
      quote! { #name: rendiation_shadergraph::ShaderGraphNodeHandle<#ty>, }
    })
    .collect();

  let gen_node_connect: Vec<_> = params
    .iter()
    .map(|(_, name)| {
      quote! { graph.nodes.connect_node(#name.cast_type(), result); }
    })
    .collect();

    // we cant use lazy_static in marco so let's try once_cell
  quote! {
    pub static #prototype_name: once_cell::sync::Lazy<rendiation_shadergraph::ShaderFunction> = 
    once_cell::sync::Lazy::new(||{
      rendiation_shadergraph::ShaderFunction{
        function_name: #quoted_function_name,
        function_source: #quoted_source,
      }
    });

    pub fn #function_name (
      #(#gen_function_inputs)*
    ) -> rendiation_shadergraph::ShaderGraphNodeHandle<#return_type> {
      use rendiation_shadergraph::*;

      let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
      let graph = guard.as_mut().unwrap();
      let result = graph
        .nodes
        .create_node(ShaderGraphNode::new(
            ShaderGraphNodeData::Function(
            FunctionNode {
              prototype: & #prototype_name
            },
          ),
          #function_node_type
        )
      );
      unsafe {
        #(#gen_node_connect)*
        result.cast_type()
      }
    }

  }
}

fn convert_type(glsl: &syntax::TypeSpecifierNonArray) -> proc_macro2::TokenStream {
  use syntax::TypeSpecifierNonArray::*;
  match glsl {
    Float => quote! { f32 },
    Vec2 => quote! { rendiation_math::Vec2<f32> },
    Vec3 => quote! { rendiation_math::Vec3<f32> },
    Vec4 => quote! { rendiation_math::Vec4<f32> },
    _ => panic!("unsupported param type"),
  }
}

fn convert_node_type(glsl: &syntax::TypeSpecifierNonArray) -> proc_macro2::TokenStream {
  use syntax::TypeSpecifierNonArray::*;
  match glsl {
    Float => quote! { NodeType::Float },
    Vec2 => quote! { NodeType::Vec2 },
    Vec3 => quote! { NodeType::Vec3 },
    Vec4 => quote! { NodeType::Vec4 },
    _ => panic!("unsupported param type"),
  }
}