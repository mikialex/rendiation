use rendiation_color::*;

use crate::*;

pub struct ShadingSelection;

pub fn srgba_to_linear(color: Vec4<f32>) -> Vec4<f32> {
  let alpha = color.a();
  let color = srgb_to_linear(color.rgb());
  Vec4::new(color.x, color.y, color.z, alpha)
}

pub fn srgb_to_linear(color: Vec3<f32>) -> Vec3<f32> {
  let color: SRGBColor<f32> = color.into();
  let color: LinearRGBColor<f32> = color.into();
  color.into()
}
