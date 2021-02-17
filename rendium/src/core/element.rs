use rendiation_algebra::{Mat4, Vec2};
use std::any::Any;

pub struct QuadLayout {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

impl QuadLayout {
  pub fn new() -> Self {
    Self {
      x: 0.,
      y: 0.,
      width: 1.,
      height: 1.,
    }
  }

  pub fn is_point_in(&self, point: Vec2<f32>) -> bool {
    point.x >= self.x
      && point.y >= self.y
      && point.x <= self.x + self.width
      && point.y <= self.y + self.height
  }

  pub fn model_matrix(&self) -> Mat4<f32> {
    let scale_mat = Mat4::scale(self.width / 2., self.height / 2., 1.0);
    let position_mat = Mat4::translate(-self.x, -self.y, 0.0);
    position_mat * scale_mat * Mat4::translate(-1., -1., 0.0)
  }
}

pub struct DIV {
  layout: QuadLayout,
}

impl DIV {
  pub fn new() -> Self {
    Self {
      layout: QuadLayout::new(),
    }
  }
}

impl Element for DIV {}

pub trait Element: Any {
  // fn event(&self, event: &mut Message);
  // fn get_element_state(&self) -> &ElementState;
  // fn is_point_in(&self, point: Vec2<f32>) -> bool;
}

pub struct ElementState {
  is_active: bool,
  is_hover: bool,
  is_focus: bool,
}

impl ElementState {
  pub fn new() -> Self {
    Self {
      is_active: false,
      is_hover: false,
      is_focus: false,
    }
  }
}
