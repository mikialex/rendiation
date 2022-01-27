pub mod wgsl;
pub use wgsl::*;

pub mod glsl;
pub use glsl::*;

use crate::*;

pub trait ShaderGraphCodeGenTarget {
  fn gen_primitive_type(&self, ty: PrimitiveShaderValueType) -> &'static str;
  fn gen_expr(
    &self,
    data: &ShaderGraphNodeData,
    builder: &mut ShaderGraphBuilder,
  ) -> Option<String>;
}
