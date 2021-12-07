use crate::{Path2dBuilder, UIPosition};

pub trait Shape {
  // fn create_path(&self, builder: &mut Path2dBuilder);
  fn triangulate_fill<T>(&self, path: &mut Vec<T>);
}

pub struct MeshBuilder<T> {
  buffer: Vec<T>,
}

impl<T> MeshBuilder<T> {
  pub fn add_triangle(&mut self, a: T, b: T, c: T) {
    self.buffer.push(a);
    self.buffer.push(b);
    self.buffer.push(c);
  }
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

#[derive(Default)]
pub struct QuadBoundaryWidth {
  pub top: f32,
  pub bottom: f32,
  pub left: f32,
  pub right: f32,
}

#[derive(Default)]
pub struct QuadBorder {
  pub radius: QuadRadius,
  pub width: QuadBoundaryWidth,
}
