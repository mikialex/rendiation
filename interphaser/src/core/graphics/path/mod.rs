use rendiation_algebra::*;
use rendiation_geometry::*;

pub mod builder;
pub use builder::*;
pub mod container;
pub use container::*;

pub enum Path2dSegment<T> {
  Line(LineSegment<Vec2<T>>),
  QuadBezier,
  CubicBezier,
}

impl<T: Scalar> Path2dSegment<T> {
  pub fn tessellate(&self, buffer: &mut TessellatedPathBuffer<T>) {
    match self {
      Path2dSegment::Line(l) => buffer.add_line_segment(*l),
      Path2dSegment::QuadBezier => todo!(),
      Path2dSegment::CubicBezier => todo!(),
    }
  }
}

impl<T: Scalar> Path2dSegment<T> {
  fn sample(&self, t: T) -> Vec2<T> {
    match self {
      Path2dSegment::Line(l) => l.sample(t),
      Path2dSegment::QuadBezier => todo!(),
      Path2dSegment::CubicBezier => todo!(),
    }
  }
}
