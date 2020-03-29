use crate::controller::Controller;
use crate::transformed_object::TransformedObject;
use rendiation_math::*;
use rendiation_math_entity::Spherical;

pub struct FPSController {
  spherical: Spherical,

  // restriction
  max_polar_angle: f32,
  min_polar_angle: f32,

  x_motion: f32, 
  y_motion: f32,
  motion_rate: f32,

  pub a_press: bool,
  pub d_press: bool,
  pub w_press: bool,
  pub s_press: bool,
  pub space_press: bool,
  pub l_shift_press: bool,
}

impl FPSController {
  pub fn new() -> Self {
    FPSController {
      spherical: Spherical::new(),
      max_polar_angle: 179. / 180. * std::f32::consts::PI,
      min_polar_angle: 0.01,

      x_motion: 0., 
      y_motion: 0.,
      motion_rate: 0.1,

      a_press: false,
      d_press: false,
      w_press: false,
      s_press: false,
      space_press: false,
      l_shift_press: false,
    }
  }
}

impl<T: TransformedObject> Controller<T> for FPSController {
  fn update(&mut self, target: &mut T) {
    let mut mat = target.get_transform_mut().matrix;
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

    let position_move = move_dir * mat;
    let position_new = mat.position() + position_move;

    self.spherical.polar = (self.spherical.polar + self.y_motion * self.motion_rate)
    .max(self.min_polar_angle)
    .min(self.max_polar_angle);
    self.spherical.azim -= self.x_motion * self.motion_rate;

    mat = Mat4::lookat(position_new, position_new + self.spherical.to_vec3(), Vec3::unit_y());
  }
}
