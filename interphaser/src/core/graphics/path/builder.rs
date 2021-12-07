use rendiation_algebra::Vec2;
use rendiation_geometry::LineSegment;
use rendiation_geometry::SpaceLineSegment;

use crate::{Path2D, Path2dSegment};

pub struct Path2dBuilder {
  path: Vec<Path2dSegment<f32>>,
  current_point: Vec2<f32>,
}

impl Default for Path2dBuilder {
  fn default() -> Self {
    Self {
      path: Default::default(),
      current_point: Vec2::new(0., 0.),
    }
  }
}

impl Path2dBuilder {
  pub fn line_to(&mut self, point: impl Into<Vec2<f32>>) -> &mut Self {
    let point = point.into();
    let curve = LineSegment::new(self.current_point, point);
    self.path.push(Path2dSegment::Line(curve));
    self.current_point = point;
    self
  }

  pub fn move_to(&mut self, point: impl Into<Vec2<f32>>) -> &mut Self {
    self.current_point = point.into();
    self
  }

  fn close_path(&mut self) {
    // check should close?
    if self.path.len() <= 2 {
      return;
    }

    let start = self.path.first().unwrap().start();
    let end = self.path.last().unwrap().end();

    // check if has closed actually
    if start != end {
      self
        .path
        .push(Path2dSegment::Line(LineSegment::new(end, start)));
    }
  }

  pub fn build(mut self, close_path: bool) -> Path2D<f32> {
    if close_path {
      self.close_path();
    }

    Path2D {
      segments: self.path,
    }
  }
}
