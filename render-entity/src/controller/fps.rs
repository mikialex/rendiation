use crate::controller::Controller;
use crate::transformed_object::TransformedObject;
use rendiation_math::*;
use rendiation_math_entity::Spherical;

struct FPSController {
  spherical: Spherical,
  a_press: bool,
  d_press: bool,
  w_press: bool,
  s_press: bool,
  space_press: bool,
  l_shift_press: bool,
}

impl FPSController {
  pub fn new() -> Self {
    FPSController {
      spherical: Spherical::new(),
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
    // cam.transform.update_matrix();
    // let mat = cam.transform.matrix;
    // let mut move_dir = Vec4::new(0.0, 0.0, 0.0, 1.0);

    // if self.w_press {
    //   move_dir.z -= 1.0;
    // }
    // if self.s_press {
    //   move_dir.z += 1.0;
    // }
    // if self.a_press {
    //   move_dir.x -= 1.0;
    // }
    // if self.d_press {
    //   move_dir.x += 1.0;
    // }
    // if self.space_press {
    //   move_dir.y += 1.0;
    // }
    // if self.l_shift_press {
    //   move_dir.y -= 1.0;
    // }

    // let position_new = mat * move_dir;

    // let position_move = Vec3::new(
    //   position_new.x / position_new.w,
    //   position_new.y / position_new.w,
    //   position_new.z / position_new.w,
    // ) - cam.transform.position;

    // let normalized = position_move.normalize() * 0.1;

    // cam.transform.position.x += normalized.x;
    // cam.transform.position.z += normalized.z;
    // cam.transform.position.y += normalized.y;

    // let rate: f32 = 0.1;
    // let mut sph = self.spherical;
    // self.spherical.polar = clamp(
    //   sph.polar - cgm::Deg(y_motion * rate),
    //   cgm::Deg(0.1),
    //   cgm::Deg(179.9),
    // );
    // sph.azim -= cgm::Deg(x_motion * rate);

    // let look_point = Vec3::new(
    //   sph.polar.sin() * sph.azim.sin(),
    //   sph.polar.cos(),
    //   sph.polar.sin() * sph.azim.cos(),
    // );

    // let transform = target.get_transform_mut();
    // transform.matrix = Mat4::lookat(eye, self.spherical.center, Vec3::unit_y());
    // self.camera.borrow_mut().transform.rotation = cgm::Rotation::look_at(look_point, CAMERA_UP);
  }
}
