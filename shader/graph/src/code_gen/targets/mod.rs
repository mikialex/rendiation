pub mod wgsl;
pub use wgsl::*;

pub mod glsl_es;
pub use glsl_es::*;

use crate::*;

pub trait ShaderGraphCodeGenTarget {
  type ShaderSource;
  fn compile(
    &self,
    builder: &ShaderGraphRenderPipelineBuilder,
    vertex: ShaderGraphBuilder,
    fragment: ShaderGraphBuilder,
  ) -> Self::ShaderSource;
}

/// common & shareable impl

pub fn float_to_shader(f: f32) -> String {
  let mut result = format!("{}", f);
  if result.contains('.') {
    result
  } else {
    result.push_str(".0");
    result
  }
}

pub fn float_group(f: &[f32]) -> String {
  let v = f
    .iter()
    .map(|f| float_to_shader(*f))
    .collect::<Vec<_>>()
    .join(", ");
  format!("({})", v)
}
