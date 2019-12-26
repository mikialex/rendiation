use rendiation_math::*;

#[derive(Default)]
pub struct Transfromation {
  pub matrix: Mat4<f32>,
  pub position: Vec3<f32>,
  pub scale: Vec3<f32>,
  pub rotation: Quat<f32>,
}

impl Transfromation {
  pub fn new() -> Self {
    Transfromation {
      position: Vec3::new(0.0, 0.0, 0.0),
      scale: Vec3::new(1.0, 1.0, 1.0),
      rotation: Quat::new(0.0, 0.0, 0.0, 1.0),
      matrix: Mat4::one(),
    }
  }

  pub fn update_matrix_by_compose(&mut self) {
    self.matrix = compose(&self.position, &self.rotation, &self.scale);
  }
}

fn compose(position: &Vec3<f32>, quaternion: &Quat<f32>, scale: &Vec3<f32>) -> Mat4<f32> {
  let x = quaternion.x;
  let y = quaternion.y;
  let z = quaternion.z;
  let w = quaternion.w;
  let x2 = x + x;
  let y2 = y + y;
  let z2 = z + z;
  let xx = x * x2;
  let xy = x * y2;
  let xz = x * z2;
  let yy = y * y2;
  let yz = y * z2;
  let zz = z * z2;
  let wx = w * x2;
  let wy = w * y2;
  let wz = w * z2;

  let sx = scale.x;
  let sy = scale.y;
  let sz = scale.z;

  Mat4::new(
    (1. - (yy + zz)) * sx,
    (xy + wz) * sx,
    (xz - wy) * sx,
    0.,
    (xy - wz) * sy,
    (1. - (xx + zz)) * sy,
    (yz + wx) * sy,
    0.,
    (xz + wy) * sz,
    (yz - wx) * sz,
    (1. - (xx + yy)) * sz,
    0.,
    position.x,
    position.y,
    position.z,
    1.,
  )
}
