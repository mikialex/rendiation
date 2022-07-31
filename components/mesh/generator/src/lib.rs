use std::{marker::PhantomData, ops::Range};

use rendiation_algebra::*;
use rendiation_renderable_mesh::mesh::{IndexedMesh, LineList, TriangleList};

pub trait ParametricSurface {
  fn sample(&self, position: Vec2<f32>) -> Vec3<f32>;
}

pub trait ParametricCurve {
  fn sample(&self, position: f32) -> Vec3<f32>;
}

pub struct ParametricRangeMapping<T> {
  pub inner: T,
  pub start: f32,
  pub end: f32,
}
pub trait IntoParametricRangeMapping: ParametricCurve + Sized {
  fn map_range(self, range: Range<f32>) -> ParametricRangeMapping<Self> {
    ParametricRangeMapping {
      inner: self,
      start: range.start,
      end: range.end,
    }
  }
}
impl<T: ParametricCurve + Sized> IntoParametricRangeMapping for T {}

pub struct Embed2DCurveTo3DSurface<S, T> {
  pub curve: S,
  pub surface: T,
}
pub trait IntoEmbed2DCurveTo3DSurface: ParametricCurve + Sized {
  fn embed_to_surface<T>(self, surface: T) -> Embed2DCurveTo3DSurface<Self, T>
  where
    T: ParametricSurface,
  {
    Embed2DCurveTo3DSurface {
      curve: self,
      surface,
    }
  }
}
impl<S: ParametricCurve + Sized> IntoEmbed2DCurveTo3DSurface for S {}

// pub struct UnitCircle;

// pub fn torus() -> {
//   let radius =1.;
//   let tri_config = TriangulateConfig {
//     u_segments: 20,
//     v_segments: 20,
//   };
//   UnitCircle.scale()
//   .embed(XYPlane)
//   .map_range_u((0., 1.))
//   .map_range_v((0., 1.))
//   .make_curve_tube(radius)
//   .triangulate(tri_config)
// }

pub struct IndexedMeshBuilder<I, U, T, V> {
  index: Vec<I>,
  container: U,
  phantom1: PhantomData<T>,
  phantom2: PhantomData<V>,
}

impl<I, U, T, V> IndexedMeshBuilder<I, U, T, V> {
  pub fn build_mesh(self) -> IndexedMesh<I, V, T, U> {
    IndexedMesh::new(self.container, self.index)
  }
}

#[derive(Copy, Clone)]
pub struct TessellationConfig {
  pub u: usize,
  pub v: usize,
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
  config: &TessellationConfig,
  builder: &mut IndexedMeshBuilder<I, U, TriangleList, V>,
) where
  V: VertexBuilding,
  U: VertexContainer<Vertex = V>,
  I: From<usize> + Copy,
{
  let u_step = 1. / config.u as f32;
  let v_step = 1. / config.v as f32;
  for u in 0..config.u {
    for v in 0..config.v {
      let u = u as f32 * u_step;
      let v = v as f32 * v_step;
      let vertex = V::from_surface(surface, (u, v).into());
      builder.container.push_vertex(vertex)
    }
  }

  let index_start = builder.index.len();
  let uv_to_index = |u: usize, v: usize| -> I { (index_start + u + config.u * v).into() };

  for u in 0..config.u {
    for v in 0..config.v {
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

pub fn build_grid_parametric<V, I, U>(
  surface: &impl ParametricSurface,
  config: &TessellationConfig,
  builder: &mut IndexedMeshBuilder<I, U, LineList, V>,
) where
  V: VertexBuilding,
  U: VertexContainer<Vertex = V>,
  I: From<usize> + Copy,
{
  let u_step = 1. / config.u as f32;
  let v_step = 1. / config.v as f32;
  for u in 0..config.u {
    for v in 0..config.v {
      let u = u as f32 * u_step;
      let v = v as f32 * v_step;
      let vertex = V::from_surface(surface, (u, v).into());
      builder.container.push_vertex(vertex)
    }
  }

  let index_start = builder.index.len();
  let uv_to_index = |u: usize, v: usize| -> I { (index_start + u + config.u * v).into() };

  for u in 0..config.u {
    for v in 0..config.v {
      // a  b
      // c  d
      let a = uv_to_index(u, v);
      let b = uv_to_index(u, v + 1);
      let c = uv_to_index(u + 1, v);
      let d = uv_to_index(u + 1, v + 1);

      builder.index.push(a);
      builder.index.push(b);

      builder.index.push(a);
      builder.index.push(c);

      if u == config.u {
        builder.index.push(c);
        builder.index.push(d);
      }

      if v == config.v {
        builder.index.push(b);
        builder.index.push(d);
      }
    }
  }
}
