use rendiation_algebra::*;
use rendiation_geometry::*;

pub struct Path2D<T> {
  segments: Vec<T>,
}

pub enum Path2dSegment<T> {
  Line(LineSegment<Vec2<T>>),
  QuadBezier,
  CubicBezier,
}

impl<T: Scalar> SpaceLineSegment<T, Vec2<T>> for Path2dSegment<T> {
  fn start(&self) -> Vec2<T> {
    match self {
      Path2dSegment::Line(_) => todo!(),
      Path2dSegment::QuadBezier => todo!(),
      Path2dSegment::CubicBezier => todo!(),
    }
  }

  fn end(&self) -> Vec2<T> {
    match self {
      Path2dSegment::Line(_) => todo!(),
      Path2dSegment::QuadBezier => todo!(),
      Path2dSegment::CubicBezier => todo!(),
    }
  }

  fn sample(&self, t: T) -> Vec2<T> {
    // match self {
    //   Path2dSegment::Line(l) => l.sample(t),
    //   Path2dSegment::QuadBezier => todo!(),
    //   Path2dSegment::CubicBezier => todo!(),
    // }
    todo!()
  }
}

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

  pub fn build(mut self, close_path: bool) -> Path2D<Path2dSegment<f32>> {
    if close_path {
      self.close_path();
    }

    Path2D {
      segments: self.path,
    }
  }
}
