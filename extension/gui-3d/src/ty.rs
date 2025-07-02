use crate::*;

#[derive(Clone, Copy)]
pub enum AxisType {
  X,
  Y,
  Z,
}

impl AxisType {
  pub fn dir(&self) -> Vec3<f64> {
    match self {
      AxisType::X => Vec3::new(1., 0., 0.),
      AxisType::Y => Vec3::new(0., 1., 0.),
      AxisType::Z => Vec3::new(0., 0., 1.),
    }
  }
  pub fn mat(&self) -> Mat4<f64> {
    match self {
      AxisType::X => Mat4::rotate_z(-f64::PI() / 2.),
      AxisType::Y => Mat4::identity(),
      AxisType::Z => Mat4::rotate_x(f64::PI() / 2.),
    }
  }
}
pub struct GlobalUIStyle {
  pub x_color: Vec3<f32>,
  pub y_color: Vec3<f32>,
  pub z_color: Vec3<f32>,
}

const RED: Vec3<f32> = Vec3::new(0.8, 0.3, 0.3);
const GREEN: Vec3<f32> = Vec3::new(0.3, 0.8, 0.3);
const BLUE: Vec3<f32> = Vec3::new(0.3, 0.3, 0.8);
impl Default for GlobalUIStyle {
  fn default() -> Self {
    Self {
      x_color: RED,
      y_color: GREEN,
      z_color: BLUE,
    }
  }
}

impl GlobalUIStyle {
  pub fn get_axis_primary_color(&self, axis: AxisType) -> Vec3<f32> {
    match axis {
      AxisType::X => self.x_color,
      AxisType::Y => self.y_color,
      AxisType::Z => self.z_color,
    }
  }
}
