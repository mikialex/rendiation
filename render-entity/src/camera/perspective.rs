use crate::Projection;
use crate::ResizableProjection;
use rendiation_math::*;

pub struct PerspectiveCamera {
  pub near: f32,
  pub far: f32,
  pub fov: f32,
  pub aspect: f32,
  pub zoom: f32,
}

impl PerspectiveCamera {
  pub fn new() -> Self {
    Self {
      near: 1.,
      far: 100_000.,
      fov: 90.,
      aspect: 1.,
      zoom: 1.,
    }
  }
}

impl Projection for PerspectiveCamera {
  fn update(&self, projection: &mut Mat4<f32>) {
    *projection = Mat4::perspective_fov_rh(self.fov, self.aspect, self.near, self.far);
  }
}

impl ResizableProjection for PerspectiveCamera {
  fn resize(&mut self, size: (f32, f32)) {
    self.aspect = size.0 / size.1;
  }
}
