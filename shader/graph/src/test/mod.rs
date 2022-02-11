mod complex;
mod function;
mod uniform;
mod varying;

use crate::*;

pub fn test_provider_success(s: &dyn ShaderGraphProvider) {
  let result = build_shader(s, &WGSL);
  test_build_result_success(result)
}

pub fn test_build_result_success(result: Result<ShaderGraphCompileResult, ShaderGraphBuildError>) {
  let result = result.unwrap();

  if let Err(e) = naga::front::wgsl::parse_str(&result.vertex_shader) {
    e.emit_to_stderr(&result.vertex_shader);
  }

  println!("=======  vertex  ======= \n{}", result.vertex_shader);

  if let Err(e) = naga::front::wgsl::parse_str(&result.frag_shader) {
    e.emit_to_stderr(&result.frag_shader);
  }

  println!("======= fragment ======= \n{}", result.frag_shader);
}
