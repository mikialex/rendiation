#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct LTCRectLightUniform {
  /// pre calculated vertex in world space.
  pub p1: Vec3<f32>,
  pub p2: Vec3<f32>,
  pub p3: Vec3<f32>,
  pub p4: Vec3<f32>,
  pub intensity: Vec3<f32>,
  pub double_side: Bool,
  pub is_disk: Bool,
}
