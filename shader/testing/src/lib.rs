mod complex;
mod function;
mod uniform;
mod varying;

use shadergraph::*;
use wgsl_codegen_graph::*;

pub fn test_provider_success(s: &dyn GraphicsShaderProvider) {
  let mut builder = Default::default();
  s.build(&mut builder).unwrap();
  let result = builder.build(WGSL);
  test_build_result_success(result)
}

pub fn test_build_result_success(
  result: Result<ShaderGraphCompileResult<WGSL>, ShaderGraphBuildError>,
) {
  let result = result.unwrap().shader;

  if let Err(e) = naga::front::wgsl::parse_str(&result.vertex) {
    e.emit_to_stderr(&result.vertex);
  }

  println!("=======  vertex  ======= \n{}", result.vertex);

  if let Err(e) = naga::front::wgsl::parse_str(&result.fragment) {
    e.emit_to_stderr(&result.fragment);
  }

  println!("======= fragment ======= \n{}", result.fragment);
}
