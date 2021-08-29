pub struct Path2D<T> {
  segments: Vec<T>,
}

pub enum Path2dSegment<T> {
  Line(LineSegment<Vec2<T>>),
  QuadBezier,
  CubicBezier,
}

pub struct Path2dBuilder {
  path: Path2D<Path2dSegment>,
}

impl Path2dBuilder {
  pub fn line_to(&mut self) -> &mut self {
    //
  }

  pub fn move_to(&mut self) -> &mut self {
    //
  }
}
