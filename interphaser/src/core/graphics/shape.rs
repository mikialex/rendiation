use crate::{UIPosition, UISize};

pub trait Shape {
  // fn create_path(&self, builder: &mut Path2dBuilder);
  fn triangulate_fill<T>(&self, path: &mut Vec<T>);
  // fn triangulate_stroke<T>(&self, path: &mut Vec<T>);
}

#[derive(Debug, Clone, Default, Copy)]
pub struct Quad {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

impl Quad {
  pub fn is_point_in(&self, p: UIPosition) -> bool {
    p.x >= self.x && p.x <= self.x + self.width && p.y >= self.y && p.y <= self.y + self.height
  }
}

#[derive(Debug, Clone, Default, Copy)]
pub struct RoundCorneredQuad {
  pub quad: Quad,
  pub radius: QuadRadius,
}

#[derive(Debug, Clone, Default, Copy)]
pub struct RadiusGroup {
  pub top_left: f32,
  pub top_right: f32,
  pub bottom_left: f32,
  pub bottom_right: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum QuadRadius {
  No,
  All(f32),
  Four(RadiusGroup),
}

impl Default for QuadRadius {
  fn default() -> Self {
    Self::No
  }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct QuadBoundaryWidth {
  pub top: f32,
  pub bottom: f32,
  pub left: f32,
  pub right: f32,
}

impl From<QuadBoundaryWidth> for UISize {
  fn from(v: QuadBoundaryWidth) -> Self {
    (v.left + v.right, v.top + v.bottom).into()
  }
}

impl UISize {
  pub fn inset_boundary(self, b: &QuadBoundaryWidth) -> Self {
    (
      (self.width - b.left - b.right).max(0.),
      (self.height - b.top - b.bottom).max(0.),
    )
      .into()
  }
}

impl QuadBoundaryWidth {
  pub fn equal(size: f32) -> Self {
    Self {
      top: size,
      bottom: size,
      left: size,
      right: size,
    }
  }
}

#[derive(Default)]
pub struct QuadBorder {
  pub radius: QuadRadius,
  pub width: QuadBoundaryWidth,
}
