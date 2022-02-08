pub mod wgsl;
pub use wgsl::*;

use crate::*;

pub trait ShaderGraphCodeGenTarget {
  fn gen_vertex_shader(
    &self,
    vertex: &mut ShaderGraphVertexBuilder,
    builder: ShaderGraphBuilder,
  ) -> String;
  fn gen_fragment_shader(
    &self,
    vertex: &mut ShaderGraphFragmentBuilder,
    builder: ShaderGraphBuilder,
  ) -> String;
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
