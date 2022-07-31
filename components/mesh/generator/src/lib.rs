use rendiation_algebra::*;

pub trait ParametricSurface {
  fn sample(&self, position: Vec2<f32>) -> Vec3<f32>;
}

pub trait ParametricCurve {
  fn sample(&self, position: f32) -> Vec3<f32>;
}

// pub struct UnitCircle;

// pub fn torus() -> {
//   let radius =1.;
//   let tri_config = TriangulateConfig {
//     u_range: (0., 1.),
//     v_range: (0., 1.),
//     u_segments: 20,
//     v_segments: 20,
//   };
//   UnitCircle.scale().embed(XYPlane).make_curve_tube(radius).triangulate(tri_config)
// }

pub struct IndexedMeshBuilder<I, U> {
  index: Vec<I>,
  container: U,
}

#[derive(Copy, Clone)]
pub struct TriangulateConfig {
  pub u: EqualSegmentsDescriptor,
  pub v: EqualSegmentsDescriptor,
}

#[derive(Copy, Clone)]
pub struct EqualSegmentsDescriptor {
  pub start: f32,
  pub end: f32,
  pub segments: usize,
}

impl IntoIterator for EqualSegmentsDescriptor {
  type Item = f32;
  type IntoIter = EqualSegmentsIter;
  fn into_iter(self) -> Self::IntoIter {
    EqualSegmentsIter {
      current: 0.,
      step: self.end - self.start,
      last: self.segments,
    }
  }
}

pub struct EqualSegmentsIter {
  current: f32,
  step: f32,
  last: usize,
}

impl Iterator for EqualSegmentsIter {
  type Item = f32;

  fn next(&mut self) -> Option<Self::Item> {
    if self.last == 0 {
      None
    } else {
      let r = self.current;
      self.current += self.step;
      self.last -= 1;
      Some(r)
    }
  }
}

pub trait VertexContainer {
  type Vertex;
  fn push_vertex(&mut self, v: Self::Vertex);
}

pub trait VertexBuilding {
  fn from_surface(surface: &impl ParametricSurface, uv: Vec2<f32>) -> Self;
}

pub fn triangulate_parametric<V, I, U>(
  surface: &impl ParametricSurface,
  config: &TriangulateConfig,
  builder: &mut IndexedMeshBuilder<I, U>,
) where
  V: VertexBuilding,
  U: VertexContainer<Vertex = V>,
  I: From<usize> + Copy,
{
  for u in config.u {
    for v in config.v {
      let vertex = V::from_surface(surface, (u, v).into());
      builder.container.push_vertex(vertex)
    }
  }

  let index_start = builder.index.len();
  let uv_to_index = |u: usize, v: usize| -> I { (index_start + u + config.u.segments * v).into() };

  for u in 0..config.u.segments {
    for v in 0..config.v.segments {
      // a  b
      // c  d
      let a = uv_to_index(u, v);
      let b = uv_to_index(u, v + 1);
      let c = uv_to_index(u + 1, v);
      let d = uv_to_index(u + 1, v + 1);

      builder.index.push(a);
      builder.index.push(c);
      builder.index.push(b);

      builder.index.push(b);
      builder.index.push(c);
      builder.index.push(d);
    }
  }
}
