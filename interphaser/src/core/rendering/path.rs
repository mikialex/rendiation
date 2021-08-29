pub struct Path2D<T> {
  segments: Vec<T>,
}

pub struct PathBuilder {
  //
}

pub enum Path2dSegment {
  Line,
  QuadBezier,
  CubicBezier,
}
