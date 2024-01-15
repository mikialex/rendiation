pub struct ShaderFrustum {
  plane: [ShaderPlane; 6],
}

pub struct ShaderPlane {
  normal: Vec3<f32>,
  constant: f32,
}

pub struct ShaderAABB {
  pub min: Vec3<f32>,
  pub max: Vec3<f32>,
}
