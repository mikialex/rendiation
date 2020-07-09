use proc_macro::TokenStream;

use syn::parse_macro_input;
use glsl::parser::Parse;
use glsl::syntax;
use quote::{format_ident, quote};

#[proc_macro]
pub fn glsl_function(input: TokenStream) -> TokenStream{
  let input = parse_macro_input!(input as syn::LitStr);
  let glsl = input.value();
  let glsl = glsl.trim_start();
  let parsed = syntax::FunctionDefinition::parse(glsl).unwrap();

  let function_name = parsed.prototype.name.as_str();

  let struct_name = format_ident!("{}Function", function_name);
  let quoted_function_name = format!("\"{}\"", function_name);
  let quoted_source = format!("\"{}\"", glsl);

  (quote! {
    use rendiation_math::*;
    use rendiation_shadergraph::*;

    #[allow(non_camel_case_types)]
    pub struct #struct_name {
      name: &'static str,
      source: &'static str,
    }

    impl StaticShaderFunction for #struct_name{
      fn name() -> &'static str{
        #quoted_function_name
      }
      fn source() -> &'static str{
        #quoted_source
      }
    }
    
    fn uncharted2ToneMapping(
      intensity: &ShaderGraphNode<Vec3<f32>>,
      toneMappingExposure: &ShaderGraphNode<f32>,
      toneMappingWhitePoint: &ShaderGraphNode<f32>,
    ) -> ShaderGraphNode<Vec3<f32>> {
      todo!()
    }
  }).into()
}