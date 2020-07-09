use proc_macro::TokenStream;

use syn::parse_macro_input;
use glsl::parser::Parse;
use glsl::syntax;
use quote::quote;

#[proc_macro]
pub fn glsl_function(input: TokenStream) -> TokenStream{
  let input = parse_macro_input!(input as syn::LitStr);
  let glsl = input.value();
  let glsl = glsl.trim_start();
  print!("{}", glsl);
  let parsed = syntax::FunctionDefinition::parse(glsl).unwrap();

  // if let Ok(tu) = parsed {
  //   // create the stream and return it
  //   let mut stream = TokenStream::new();
  //   tu.tokenize(&mut stream);

  //   stream.into()
  // } else {
  //   panic!("GLSL error: {:?}", parsed);
  // }
  (quote! {
    use rendiation_math::*;
    use rendiation_shadergraph::*;

    // #[allow(non_camel_case_types)]
    pub struct uncharted2ToneMappingFunction {
      name: &'static str,
      source: &'static str,
    }
    
    // fn uncharted2ToneMapping(
    //   intensity: &ShaderGraphNode<Vec3<f32>>,
    //   toneMappingExposure: &ShaderGraphNode<f32>,
    //   toneMappingWhitePoint: &ShaderGraphNode<f32>,
    // ) -> ShaderGraphNode<Vec3<f32>> {
    //   todo!()
    // }
  }).into()
}