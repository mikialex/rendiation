use rendiation_algebra::*;
use rendiation_geometry::Spherical;

use crate::{Controller, Transformed3DControllee};

pub struct FPSController {
  spherical: Spherical,

  // restriction
  max_polar_angle: f32,
  min_polar_angle: f32,

  view_width: f32,
  view_height: f32,
  rotate_angle_factor: f32,

  pub leftward_active: bool,
  pub rightward_active: bool,
  pub forward_active: bool,
  pub backward_active: bool,
  pub ascend_active: bool,
  pub descend_active: bool,
}

impl Default for FPSController {
  fn default() -> Self {
    Self::new()
  }
}

impl FPSController {
  pub fn new() -> Self {
    let mut spherical = Spherical::new();
    spherical.polar = 1.;
    spherical.azim = 1.;
    FPSController {
      spherical,
      max_polar_angle: 179. / 180. * std::f32::consts::PI,
      min_polar_angle: 0.01,

      view_width: 1000.,
      view_height: 1000.,
      rotate_angle_factor: 0.5,

      leftward_active: false,
      rightward_active: false,
      forward_active: false,
      backward_active: false,
      ascend_active: false,
      descend_active: false,
    }
  }

  pub fn rotate(&mut self, offset: Vec2<f32>) {
    use std::f32::consts::PI;
    self.spherical.polar += offset.y / self.view_height * PI * self.rotate_angle_factor;
    self.spherical.polar = self
      .spherical
      .polar
      .max(self.min_polar_angle)
      .min(self.max_polar_angle);
    self.spherical.azim += offset.x / self.view_width * PI * self.rotate_angle_factor;
  }
}

impl<T: Transformed3DControllee> Controller<T> for FPSController {
  fn update(&mut self, target: &mut T) -> bool {
    let mat = target.matrix_mut();
    let mut move_dir = Vec3::new(0.0, 0.0, 0.0);

    if self.forward_active {
      move_dir.z -= 1.0;
    }
    if self.backward_active {
      move_dir.z += 1.0;
    }
    if self.leftward_active {
      move_dir.x -= 1.0;
    }
    if self.rightward_active {
      move_dir.x += 1.0;
    }
    if self.ascend_active {
      move_dir.y += 1.0;
    }
    if self.descend_active {
      move_dir.y -= 1.0;
    }

    if move_dir.length() > 0.01 {
      let position_new = move_dir * *mat;
      let position_dir = (position_new - mat.position()).normalize();
      let position_new = mat.position() + position_dir;

      *mat = Mat4::lookat(
        position_new,
        position_new + self.spherical.to_vec3(),
        Vec3::new(0.0, 1.0, 0.0),
      );
    } else {
      *mat = Mat4::lookat(
        mat.position(),
        mat.position() + self.spherical.to_vec3(),
        Vec3::new(0.0, 1.0, 0.0),
      );
    }

    true
  }
}
