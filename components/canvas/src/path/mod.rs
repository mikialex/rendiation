use rendiation_algebra::*;
use rendiation_geometry::*;

pub mod builder;
pub use builder::*;

pub enum Path2dSegment<T> {
  Line(LineSegment<Vec2<T>>),
  QuadBezier,
  CubicBezier,
}

pub struct Path2D<T> {
  pub segments: Vec<Path2dSegment<T>>,
}

impl<T: Scalar> Path2dSegment<T> {
  pub fn sample(&self, t: T) -> Vec2<T> {
    match self {
      Path2dSegment::Line(l) => l.sample(t),
      Path2dSegment::QuadBezier => todo!(),
      Path2dSegment::CubicBezier => todo!(),
    }
  }

  pub fn start(&self) -> Vec2<T> {
    match self {
      Path2dSegment::Line(l) => l.start,
      Path2dSegment::QuadBezier => todo!(),
      Path2dSegment::CubicBezier => todo!(),
    }
  }

  pub fn end(&self) -> Vec2<T> {
    match self {
      Path2dSegment::Line(l) => l.end,
      Path2dSegment::QuadBezier => todo!(),
      Path2dSegment::CubicBezier => todo!(),
    }
  }
}
