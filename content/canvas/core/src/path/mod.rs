use rendiation_algebra::*;
use rendiation_geometry::*;

mod builder;
pub use builder::*;

pub enum Path2dType<T> {
  Line(StraitLine<Vec2<T>>),
  QuadraticBezier(QuadraticBezierShape<Vec2<T>>),
  CubicBezier(CubicBezierShape<Vec2<T>>),
}

pub struct PartialPathSegment {
  pub path: Path2dType<f32>,
  pub end_point: Vec2<f32>,
}

pub struct Path2dSegments {
  pub start: Vec2<f32>,
  pub paths: Vec<PartialPathSegment>,
  pub closed: bool,
}

pub struct Path2dSegmentsGroup {
  pub sub_paths: Vec<Path2dSegments>, // todo flatten?
}
