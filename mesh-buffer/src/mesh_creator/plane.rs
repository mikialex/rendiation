use super::IndexedBufferMesher;
use crate::vertex::Vertex;
use rendiation_math::*;

pub struct Quad;

impl IndexedBufferMesher for Quad {
  fn create_mesh(&self) -> (Vec<Vertex>, Vec<u16>) {
    PlaneGeometryParameter {
      width: 2.,
      height: 2.,
      width_segments: 1,
      height_segments: 1,
    }
    .create_mesh()
  }
}

#[derive(Copy, Clone, Debug)]
pub struct PlaneGeometryParameter {
  pub width: f32,
  pub height: f32,
  pub width_segments: usize,
  pub height_segments: usize,
}

impl Default for PlaneGeometryParameter {
  fn default() -> Self {
    Self {
      width: 1.0,
      height: 1.0,
      width_segments: 1,
      height_segments: 1,
    }
  }
}

impl IndexedBufferMesher for PlaneGeometryParameter {
  fn create_mesh(&self) -> (Vec<Vertex>, Vec<u16>) {
    let Self {
      width,
      height,
      width_segments,
      height_segments,
    } = *self;

    let width_half = width / 2.0;
    let height_half = height / 2.0;
    let grid_x = if width_segments == 0 {
      1
    } else {
      width_segments
    };
    let grid_y = if height_segments == 0 {
      1
    } else {
      height_segments
    };
    let grid_x1 = grid_x + 1;
    let grid_y1 = grid_y + 1;
    let segment_width = width / grid_x as f32;
    let segment_height = height / grid_y as f32;

    let mut vertices = vec![];
    for iy in 0..grid_y1 {
      let y = iy as f32 * segment_height - height_half;
      for ix in 0..grid_x1 {
        let x = ix as f32 * segment_width - width_half;
        let position = Vec3::new(x, -y, 0.0);
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let uv = Vec2::new(ix as f32 / grid_x as f32, 1.0 - (iy as f32 / grid_y as f32));
        let vertex = Vertex::new(position, normal, uv);
        vertices.push(vertex);
      }
    }

    let mut indices = vec![];
    for iy in 0..grid_y {
      for ix in 0..grid_x {
        let a = ix + grid_x1 * iy;
        let b = ix + grid_x1 * (iy + 1);
        let c = (ix + 1) + grid_x1 * (iy + 1);
        let d = (ix + 1) + grid_x1 * iy;
        indices.push(a as u16);
        indices.push(b as u16);
        indices.push(d as u16);

        indices.push(b as u16);
        indices.push(c as u16);
        indices.push(d as u16);
      }
    }

    (vertices, indices)
  }
}
