pub mod code_builder;
pub use code_builder::*;

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
  let mut result = format!("{f}");
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
  format!("({v})")
}

pub fn uint_group(f: &[u32]) -> String {
  let v = f
    .iter()
    .map(|f| format!("{f}u"))
    .collect::<Vec<_>>()
    .join(", ");
  format!("({v})")
}
