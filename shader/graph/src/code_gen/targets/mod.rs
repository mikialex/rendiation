pub mod wgsl;
pub use wgsl::*;

pub mod glsl;
pub use glsl::*;

use crate::*;

pub trait ShaderGraphCodeGenTarget {
  fn gen_primitive_literal(&self, v: PrimitiveShaderValue) -> String;
  fn gen_primitive_type(&self, ty: PrimitiveShaderValueType) -> &'static str;
  fn gen_expr(
    &self,
    data: &ShaderGraphNodeData,
    builder: &mut ShaderGraphBuilder,
  ) -> Option<String>;
}

pub fn float_to_shader(f: f32) -> String {
  let mut result = format!("{}", f);
  if result.contains('.') {
    result
  } else {
    result.push_str(".0");
    result
  }
}
