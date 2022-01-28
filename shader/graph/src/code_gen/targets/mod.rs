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

pub fn float_group(f: &[f32]) -> String {
  let v = f
    .iter()
    .map(|f| float_to_shader(*f))
    .collect::<Vec<_>>()
    .join(", ");
  format!("({})", v)
}

pub fn gen_primitive_literal_common<T: ShaderGraphCodeGenTarget>(
  target: &T,
  v: PrimitiveShaderValue,
) -> String {
  let grouped = match v {
    PrimitiveShaderValue::Bool(v) => format!("{v}"),
    PrimitiveShaderValue::Float32(f) => return float_to_shader(f),
    PrimitiveShaderValue::Vec2Float32(v) => {
      let v: &[f32; 2] = v.as_ref();
      float_group(v.as_slice())
    }
    PrimitiveShaderValue::Vec3Float32(v) => {
      let v: &[f32; 3] = v.as_ref();
      float_group(v.as_slice())
    }
    PrimitiveShaderValue::Vec4Float32(v) => {
      let v: &[f32; 4] = v.as_ref();
      float_group(v.as_slice())
    }
    PrimitiveShaderValue::Mat2Float32(v) => {
      let v: &[f32; 4] = v.as_ref();
      float_group(v.as_slice())
    }
    PrimitiveShaderValue::Mat3Float32(v) => {
      let v: &[f32; 9] = v.as_ref();
      float_group(v.as_slice())
    }
    PrimitiveShaderValue::Mat4Float32(v) => {
      let v: &[f32; 16] = v.as_ref();
      float_group(v.as_slice())
    }
    PrimitiveShaderValue::Uint32(_) => todo!(),
  };
  format!("{}{}", target.gen_primitive_type(v.into()), grouped)
}
