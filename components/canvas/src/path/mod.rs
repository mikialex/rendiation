use rendiation_algebra::*;
use rendiation_geometry::*;

pub mod builder;
pub use builder::*;

pub enum Path2dSegment<T> {
  Line(LineSegment2D<T>),
  QuadraticBezier(QuadraticBezierSegment2D<T>),
  CubicBezier(CubicBezierSegment2D<T>),
}

pub struct Path2D<T> {
  pub segments: Vec<Path2dSegment<T>>,
}

impl<T: Scalar> Path2dSegment<T> {
  pub fn sample(&self, t: T) -> Vec2<T> {
    match self {
      Path2dSegment::Line(l) => l.sample(t),
      Path2dSegment::QuadraticBezier(l) => l.sample(t),
      Path2dSegment::CubicBezier(l) => l.sample(t),
    }
  }

  pub fn start(&self) -> Vec2<T> {
    match self {
      Path2dSegment::Line(l) => l.start,
      Path2dSegment::QuadraticBezier(l) => l.start,
      Path2dSegment::CubicBezier(l) => l.start,
    }
  }

  pub fn end(&self) -> Vec2<T> {
    match self {
      Path2dSegment::Line(l) => l.end,
      Path2dSegment::QuadraticBezier(l) => l.end,
      Path2dSegment::CubicBezier(l) => l.end,
    }
  }
}
