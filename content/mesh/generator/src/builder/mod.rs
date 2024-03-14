use rendiation_mesh_core::vertex::Vertex;

use crate::*;

mod container;

pub struct IndexedMeshBuilder<T> {
  mesh: T,
  vertex_count: usize,
}

impl<T: Default> Default for IndexedMeshBuilder<T> {
  fn default() -> Self {
    Self {
      mesh: Default::default(),
      vertex_count: 0,
    }
  }
}

impl<T> IndexedMeshBuilder<T> {
  pub fn finish(self) -> T {
    self.mesh
  }

  pub fn building_mesh(&self) -> &T {
    &self.mesh
  }
}

#[derive(Copy, Clone)]
pub struct TessellationConfig {
  pub u: usize,
  pub v: usize,
}

pub trait VertexBuildingContainer {
  type Vertex;
  fn push_vertex(&mut self, v: Self::Vertex);
  fn reserve(&mut self, _additional: usize) {}
}

pub trait IndexedBuildingContainer {
  fn push_index(&mut self, index: usize);
  fn reserve(&mut self, _additional: usize) {}
}

pub trait GroupBuildingContainer {
  fn push_consequent(&mut self, count: usize);
  fn extend_last(&mut self, count: usize);
}

impl VertexBuilding for Vertex {
  fn from_surface(surface: &impl ParametricSurface, uv: Vec2<f32>) -> Self {
    Self {
      position: surface.position(uv),
      normal: surface.normal(uv),
      uv,
    }
  }
}

/// Expressing some type can be constructed from parametric surface
pub trait VertexBuilding {
  fn from_surface(surface: &impl ParametricSurface, uv: Vec2<f32>) -> Self;
}

// todo how do we make topology safe and make grouping optional?
impl<T> IndexedMeshBuilder<T> {
  pub fn triangulate_parametric(
    &mut self,
    surface: &impl ParametricSurface,
    config: TessellationConfig,
    keep_grouping: bool,
  ) -> &mut Self
  where
    T: VertexBuildingContainer + IndexedBuildingContainer + GroupBuildingContainer,
    T::Vertex: VertexBuilding,
  {
    let index_start = self.vertex_count;
    let u_step = 1. / config.u as f32;
    let v_step = 1. / config.v as f32;
    for u in 0..=config.u {
      for v in 0..=config.v {
        let u = u as f32 * u_step;
        let v = v as f32 * v_step;
        let vertex = T::Vertex::from_surface(surface, (u, v).into());
        self.mesh.push_vertex(vertex);
        self.vertex_count += 1;
      }
    }

    let uv_to_index = |u: usize, v: usize| -> usize { index_start + v + (config.v + 1) * u };

    for u in 0..config.u {
      for v in 0..config.v {
        // a  b
        // c  d
        let a = uv_to_index(u, v);
        let b = uv_to_index(u, v + 1);
        let c = uv_to_index(u + 1, v);
        let d = uv_to_index(u + 1, v + 1);

        self.mesh.push_index(a);
        self.mesh.push_index(c);
        self.mesh.push_index(b);

        self.mesh.push_index(b);
        self.mesh.push_index(c);
        self.mesh.push_index(d);
      }
    }

    let count = config.u * config.v * 6;
    if keep_grouping {
      self.mesh.push_consequent(count);
    } else {
      self.mesh.extend_last(count)
    }

    self
  }
}

#[test]
fn triangulate() {
  use rendiation_mesh_core::{
    CollectionSize, DynIndexContainer, GroupedMesh, IndexedMesh, TriangleList,
  };
  let mut builder = IndexedMeshBuilder::<
    GroupedMesh<IndexedMesh<TriangleList, Vec<Vertex>, DynIndexContainer>>,
  >::default();
  builder.triangulate_parametric(&ParametricPlane, TessellationConfig { u: 1, v: 1 }, true);
  let mesh = builder.building_mesh();
  assert_eq!(mesh.mesh.index.len(), 6);
  assert_eq!(mesh.mesh.vertex.len(), 4);
  builder.triangulate_parametric(&ParametricPlane, TessellationConfig { u: 1, v: 1 }, true);
  let mesh = builder.building_mesh();
  assert_eq!(mesh.mesh.index.len(), 6 + 6);
  assert_eq!(mesh.mesh.vertex.len(), 4 + 4);
  builder.triangulate_parametric(&ParametricPlane, TessellationConfig { u: 2, v: 3 }, true);
  let mesh = builder.building_mesh();
  assert_eq!(mesh.mesh.index.len(), 6 + 6 + 36);
  assert_eq!(mesh.mesh.vertex.len(), 4 + 4 + 12);
}

impl<T> IndexedMeshBuilder<T> {
  pub fn build_grid_parametric(
    &mut self,
    surface: &impl ParametricSurface,
    config: TessellationConfig,
    keep_grouping: bool,
  ) -> &mut Self
  where
    T: VertexBuildingContainer + IndexedBuildingContainer + GroupBuildingContainer,
    T::Vertex: VertexBuilding,
  {
    let index_start = self.vertex_count;
    let u_step = 1. / config.u as f32;
    let v_step = 1. / config.v as f32;
    for u in 0..config.u {
      for v in 0..config.v {
        let u = u as f32 * u_step;
        let v = v as f32 * v_step;
        let vertex = T::Vertex::from_surface(surface, (u, v).into());
        self.mesh.push_vertex(vertex);
        self.vertex_count += 1;
      }
    }

    let uv_to_index = |u: usize, v: usize| -> usize { index_start + u + config.u * v };

    for u in 0..config.u {
      for v in 0..config.v {
        // a  b
        // c  d
        let a = uv_to_index(u, v);
        let b = uv_to_index(u, v + 1);
        let c = uv_to_index(u + 1, v);
        let d = uv_to_index(u + 1, v + 1);

        self.mesh.push_index(a);
        self.mesh.push_index(b);

        self.mesh.push_index(a);
        self.mesh.push_index(c);

        if u == config.u {
          self.mesh.push_index(c);
          self.mesh.push_index(d);
        }

        if v == config.v {
          self.mesh.push_index(b);
          self.mesh.push_index(d);
        }
      }
    }
    let count = config.u * config.v * 4 + config.u * 2 + config.v * 2;
    if keep_grouping {
      self.mesh.push_consequent(count);
    } else {
      self.mesh.extend_last(count)
    }

    self
  }
}
