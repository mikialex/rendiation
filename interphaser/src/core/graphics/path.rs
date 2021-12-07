use rendiation_algebra::*;
use rendiation_geometry::*;

pub struct Path2D<T> {
  pub segments: Vec<Path2dSegment<T>>,
}

pub struct LoopPath2D<T> {
  pub segments: Vec<Path2dSegment<T>>,
}

pub struct ConvexLoopPath2D<T> {
  pub segments: Vec<Path2dSegment<T>>,
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

pub struct TessellatedPathBuffer<T> {
  segments: Vec<LineSegment<Vec2<T>>>,
  is_convex: bool,
}

impl<T: Scalar> TessellatedPathBuffer<T> {
  pub fn reset(&mut self) {
    self.segments.clear();
    self.is_convex = false;
  }

  pub fn add_line_segment(&mut self, line: LineSegment<Vec2<T>>) {
    self.segments.push(line);
  }

  pub fn triangulate_fill(&self, mesh_builder: &mut MeshBuilder<Vec2<T>>) {
    if self.is_convex {
      let start_point = self.segments[0].start;
      for segment in self.segments.get(1..).unwrap().iter() {
        mesh_builder.add_triangle(start_point, segment.start, segment.end);
      }
    } else {
      todo!()
    }
  }
}

impl<T: Scalar> ConvexLoopPath2D<T> {
  pub fn triangulate_fill(&self, buffer: &mut TessellatedPathBuffer<T>) {
    buffer.reset();
    buffer.is_convex = true;
    for segment in &self.segments {
      segment.tessellate(buffer)
    }
  }
}

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
    match self {
      Path2dSegment::Line(l) => l.sample(t),
      Path2dSegment::QuadBezier => todo!(),
      Path2dSegment::CubicBezier => todo!(),
    }
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

  pub fn build(mut self, close_path: bool) -> Path2D<f32> {
    if close_path {
      self.close_path();
    }

    Path2D {
      segments: self.path,
    }
  }
}
