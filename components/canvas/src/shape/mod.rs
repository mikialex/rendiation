use crate::*;

pub enum Shape {
  Rect(RectangleShape),
  RoundCorneredRect(RoundCorneredRectangleShape),
  Path(Path2D<f32>),
}

#[derive(Debug, Clone, Default, Copy)]
pub struct RectangleShape {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

impl RectangleShape {
  pub fn is_point_in(&self, p: impl Into<Vec2<f32>>) -> bool {
    let p = p.into();
    p.x >= self.x && p.x <= self.x + self.width && p.y >= self.y && p.y <= self.y + self.height
  }
}

#[derive(Debug, Clone, Default, Copy)]
pub struct RoundCorneredRectangleShape {
  pub rect: RectangleShape,
  pub radius: CornerRadius,
}

#[derive(Debug, Clone, Default, Copy)]
pub struct RadiusGroup {
  pub top_left: f32,
  pub top_right: f32,
  pub bottom_left: f32,
  pub bottom_right: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum CornerRadius {
  #[default]
  No,
  All(f32),
  Four(RadiusGroup),
}

#[derive(Default, Debug, Clone, Copy)]
pub struct RectBoundaryWidth {
  pub top: f32,
  pub bottom: f32,
  pub left: f32,
  pub right: f32,
}

impl RectBoundaryWidth {
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
pub struct RectBorder {
  pub radius: CornerRadius,
  pub width: RectBoundaryWidth,
}
