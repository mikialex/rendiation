use crate::controller::Controller;
use crate::transformed_object::TransformedObject;
use rendiation_math::Vec2;
use rendiation_math::*;
use rendiation_math_entity::Spherical;

pub struct OrbitController {
  pub spherical: Spherical,

  rotate_angle_factor: f32,
  pan_factor: f32,
  zoom_factor: f32,

  // restriction
  max_polar_angle: f32,
  min_polar_angle: f32,

  // damping
  spherical_delta: Spherical,
  zooming: f32,
  pan_offset: Vec3<f32>,

  enable_damping: bool,
  zooming_damping_factor: f32,
  rotate_damping_factor: f32,
  pan_damping_factor: f32,

  view_width: f32,
  view_height: f32,
}

impl Default for OrbitController {
  fn default() -> Self {
    Self::new()
  }
}

impl OrbitController {
  pub fn new() -> Self {
    Self {
      spherical: Spherical::new(),

      rotate_angle_factor: 0.2,
      pan_factor: 0.0002,
      zoom_factor: 0.3,

      // restriction
      max_polar_angle: 179. / 180. * std::f32::consts::PI,
      min_polar_angle: 0.1,

      // damping
      spherical_delta: Spherical::new(),
      zooming: 1.0,
      pan_offset: Vec3::new(0.0, 0.0, 0.0),

      enable_damping: true,
      zooming_damping_factor: 0.1,
      rotate_damping_factor: 0.1,
      pan_damping_factor: 0.1,

      view_width: 1000.,
      view_height: 1000.,
    }
  }

  pub fn pan(&mut self, offset: Vec2<f32>) {
    let mut offset = offset.rotate(Vector::zero(), -self.spherical.azim);
    offset *= self.spherical.radius * self.pan_factor;
    self.pan_offset.x += offset.x;
    self.pan_offset.z += offset.y;
  }

  pub fn zoom(&mut self, factor: f32) {
    self.zooming = 1. + (factor - 1.) * self.zoom_factor;
  }

  pub fn rotate(&mut self, offset: Vec2<f32>) {
    self.spherical_delta.polar +=
      offset.y / self.view_height * std::f32::consts::PI * self.rotate_angle_factor;
    self.spherical_delta.azim +=
      offset.x / self.view_width * std::f32::consts::PI * self.rotate_angle_factor;
  }
}

impl<T: TransformedObject> Controller<T> for OrbitController {
  fn update(&mut self, target: &mut T) -> bool {
    if self.spherical_delta.azim.abs() < 0.0001
      && self.spherical_delta.polar.abs() < 0.0001
      && self.spherical_delta.radius.abs() < 0.0001
      && (self.zooming - 1.).abs() < 0.0001
      && self.pan_offset.length2() < 0.000_000_1
    {
      return false;
    }

    self.spherical.radius *= self.zooming;

    self.spherical.azim += self.spherical_delta.azim;

    self.spherical.polar = (self.spherical.polar + self.spherical_delta.polar)
      .max(self.min_polar_angle)
      .min(self.max_polar_angle);

    self.spherical.center += self.pan_offset;

    let matrix = target.matrix_mut();
    let eye = self.spherical.to_vec3();
    *matrix = Mat4::lookat(eye, self.spherical.center, Vec3::new(0.0, 1.0, 0.0));

    // update damping effect
    if self.enable_damping {
      self.spherical_delta.azim *= 1. - self.rotate_damping_factor;
      self.spherical_delta.polar *= 1. - self.rotate_damping_factor;
      self.zooming += (1. - self.zooming) * self.zooming_damping_factor;
      self.pan_offset *= 1. - self.pan_damping_factor;
    } else {
      self.spherical_delta.reset_pose();
      self.zooming = 1.;
      self.pan_offset = Vec3::zero();
    }
    true
  }
}
