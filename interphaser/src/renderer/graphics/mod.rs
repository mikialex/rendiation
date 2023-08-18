use rendiation_algebra::*;
use rendiation_shader_api::*;

#[repr(C)]
#[derive(Debug, Copy, Clone, ShaderVertex, Zeroable, Pod)]
pub struct UIVertex {
  #[semantic(GeometryPosition2D)]
  pub position: Vec2<f32>,
  #[semantic(GeometryUV)]
  pub uv: Vec2<f32>,
  #[semantic(GeometryColorWithAlpha)]
  pub color: Vec4<f32>,
}

pub fn vertex(position: (f32, f32), uv: (f32, f32), color: (f32, f32, f32, f32)) -> UIVertex {
  UIVertex {
    position: position.into(),
    uv: uv.into(),
    color: color.into(),
  }
}
