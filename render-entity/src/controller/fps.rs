use crate::controller::Controller;
use crate::transformed_object::TransformedObject;
use rendiation_math::*;
use rendiation_math_entity::Spherical;

pub struct FPSController {
  spherical: Spherical,

  // restriction
  max_polar_angle: f32,
  min_polar_angle: f32,

  view_width: f32,
  view_height: f32,
  rotate_angle_factor: f32,

  pub a_press: bool,
  pub d_press: bool,
  pub w_press: bool,
  pub s_press: bool,
  pub space_press: bool,
  pub l_shift_press: bool,
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

      a_press: false,
      d_press: false,
      w_press: false,
      s_press: false,
      space_press: false,
      l_shift_press: false,
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

impl<T: TransformedObject> Controller<T> for FPSController {
  fn update(&mut self, target: &mut T) -> bool {
    let mut transform = target.get_transform_mut();
    let mat = transform.matrix;
    let mut move_dir = Vec3::new(0.0, 0.0, 0.0);

    if self.w_press {
      move_dir.z -= 1.0;
    }
    if self.s_press {
      move_dir.z += 1.0;
    }
    if self.a_press {
      move_dir.x -= 1.0;
    }
    if self.d_press {
      move_dir.x += 1.0;
    }
    if self.space_press {
      move_dir.y += 1.0;
    }
    if self.l_shift_press {
      move_dir.y -= 1.0;
    }

    if move_dir.length() > 0.01 {
      let position_new = move_dir * mat;
      let position_dir = (position_new - mat.position()).normalize();
      let position_new = mat.position() + position_dir;

      transform.matrix = Mat4::lookat(
        position_new,
        position_new + self.spherical.to_vec3(),
        Vec3::unit_y(),
      );
    } else {
      transform.matrix = Mat4::lookat(
        mat.position(),
        mat.position() + self.spherical.to_vec3(),
        Vec3::unit_y(),
      );
    }

    true
  }
}
