use rendiation_renderable_mesh::*;

use crate::*;

pub struct IndexedMeshBuilder<T, U> {
  index: DynIndexContainer,
  container: U,
  phantom: PhantomData<T>,
  groups: MeshGroupsInfo,
}

impl<T, U> IndexedMeshBuilder<T, U> {
  pub fn build_mesh(self) -> GroupedMesh<IndexedMesh<T, U, DynIndexContainer>> {
    let mesh = IndexedMesh::new(self.container, self.index);
    GroupedMesh::new(mesh, self.groups)
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
}

/// Expressing some type can be constructed from parametric surface
pub trait VertexBuilding {
  fn from_surface(surface: &impl ParametricSurface, uv: Vec2<f32>) -> Self;
}

impl<U> IndexedMeshBuilder<TriangleList, U> {
  pub fn triangulate_parametric(
    &mut self,
    surface: &impl ParametricSurface,
    config: TessellationConfig,
    keep_grouping: bool,
  ) where
    U: VertexBuildingContainer,
    U::Vertex: VertexBuilding,
  {
    let u_step = 1. / config.u as f32;
    let v_step = 1. / config.v as f32;
    for u in 0..config.u {
      for v in 0..config.v {
        let u = u as f32 * u_step;
        let v = v as f32 * v_step;
        let vertex = U::Vertex::from_surface(surface, (u, v).into());
        self.container.push_vertex(vertex)
      }
    }

    let index_start = self.index.len();
    let uv_to_index = |u: usize, v: usize| -> usize { index_start + u + config.u * v };

    for u in 0..config.u {
      for v in 0..config.v {
        // a  b
        // c  d
        let a = uv_to_index(u, v);
        let b = uv_to_index(u, v + 1);
        let c = uv_to_index(u + 1, v);
        let d = uv_to_index(u + 1, v + 1);

        self.index.push_index_clamped_u32(a);
        self.index.push_index_clamped_u32(c);
        self.index.push_index_clamped_u32(b);

        self.index.push_index_clamped_u32(b);
        self.index.push_index_clamped_u32(c);
        self.index.push_index_clamped_u32(d);
      }
    }

    let count = config.u * config.v * 6;
    if keep_grouping {
      self.groups.push_consequent(count);
    } else {
      self.groups.extend_last(count)
    }
  }
}

impl<U> IndexedMeshBuilder<LineList, U> {
  pub fn build_grid_parametric(
    &mut self,
    surface: &impl ParametricSurface,
    config: TessellationConfig,
    keep_grouping: bool,
  ) where
    U: VertexBuildingContainer,
    U::Vertex: VertexBuilding,
  {
    let u_step = 1. / config.u as f32;
    let v_step = 1. / config.v as f32;
    for u in 0..config.u {
      for v in 0..config.v {
        let u = u as f32 * u_step;
        let v = v as f32 * v_step;
        let vertex = U::Vertex::from_surface(surface, (u, v).into());
        self.container.push_vertex(vertex)
      }
    }

    let index_start = self.index.len();
    let uv_to_index = |u: usize, v: usize| -> usize { index_start + u + config.u * v };

    for u in 0..config.u {
      for v in 0..config.v {
        // a  b
        // c  d
        let a = uv_to_index(u, v);
        let b = uv_to_index(u, v + 1);
        let c = uv_to_index(u + 1, v);
        let d = uv_to_index(u + 1, v + 1);

        self.index.push_index_clamped_u32(a);
        self.index.push_index_clamped_u32(b);

        self.index.push_index_clamped_u32(a);
        self.index.push_index_clamped_u32(c);

        if u == config.u {
          self.index.push_index_clamped_u32(c);
          self.index.push_index_clamped_u32(d);
        }

        if v == config.v {
          self.index.push_index_clamped_u32(b);
          self.index.push_index_clamped_u32(d);
        }
      }
    }
    let count = config.u * config.v * 4 + config.u * 2 + config.v * 2;
    if keep_grouping {
      self.groups.push_consequent(count);
    } else {
      self.groups.extend_last(count)
    }
  }
}
