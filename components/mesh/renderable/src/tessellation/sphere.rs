use super::{GroupedMesh, IndexedMeshTessellator};
use crate::{
  mesh::{IndexedMesh, TriangleList},
  vertex::Vertex,
};
use rendiation_algebra::*;

#[derive(Copy, Clone, Debug)]
pub struct SphereMeshParameter {
  pub radius: f32,
  pub width_segments: usize,
  pub height_segments: usize,
  pub phi_start: f32,
  pub phi_length: f32,
  pub theta_start: f32,
  pub theta_length: f32,
  pub theta_end: f32,
}

impl Default for SphereMeshParameter {
  fn default() -> Self {
    Self {
      radius: 1.0,
      width_segments: 12,
      height_segments: 12,
      phi_start: 0.,
      phi_length: std::f32::consts::PI * 2.,
      theta_start: 0.,
      theta_length: std::f32::consts::PI,
      theta_end: std::f32::consts::PI * 2.,
    }
  }
}

impl IndexedMeshTessellator for SphereMeshParameter {
  fn tessellate(&self) -> GroupedMesh<IndexedMesh<TriangleList, Vec<Vertex>, Vec<u16>>> {
    let Self {
      radius,
      width_segments,
      height_segments,
      phi_start,
      phi_length,
      theta_start,
      theta_length,
      theta_end,
    } = *self;

    let mut index = 0;
    let mut grid = vec![];

    let mut vertices = vec![];
    for iy in 0..=height_segments {
      let mut vertices_row = vec![];
      let v = iy as f32 / height_segments as f32;
      for ix in 0..=width_segments {
        let u = ix as f32 / width_segments as f32;
        let position = Vec3::new(
          -radius * (phi_start + u * phi_length).cos() * (theta_start + v * theta_length).sin(),
          radius * (theta_start + v * theta_length).cos(),
          radius * (phi_start + u * phi_length).sin() * (theta_start + v * theta_length).sin(),
        );
        let normal = position.normalize();
        let uv = Vec2::new(u, 1. - v);
        let vertex = Vertex::new(position, normal, uv);
        vertices.push(vertex);
        vertices_row.push(index);
        index += 1;
      }
      grid.push(vertices_row);
    }

    let mut indices = vec![];
    for iy in 0..height_segments {
      for ix in 0..width_segments {
        let a = grid[iy][ix + 1];
        let b = grid[iy][ix];
        let c = grid[iy + 1][ix];
        let d = grid[iy + 1][ix + 1];
        if iy != 0 || theta_start > 0. {
          indices.push(a);
          indices.push(b);
          indices.push(d);
        }
        if iy != height_segments - 1 || theta_end < std::f32::consts::PI {
          indices.push(b);
          indices.push(c);
          indices.push(d);
        }
      }
    }

    GroupedMesh::full(IndexedMesh::new(vertices, indices))
  }
}
