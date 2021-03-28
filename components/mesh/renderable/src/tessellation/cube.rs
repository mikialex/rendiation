use rendiation_algebra::{Vec2, Vec3, Vector};

use crate::{
  geometry::{IndexedGeometry, TriangleList},
  range::GeometryRangesInfo,
  vertex::Vertex,
};

use super::{IndexedGeometryTessellator, TesselationResult};

#[derive(Copy, Clone, Debug)]
pub struct CubeGeometryParameter {
  pub width: f32,
  pub height: f32,
  pub depth: f32,
  pub width_segment: usize,
  pub height_segment: usize,
  pub depth_segment: usize,
}

#[rustfmt::skip]
impl IndexedGeometryTessellator for CubeGeometryParameter {
  fn tessellate(&self) ->  TesselationResult<IndexedGeometry<u16, Vertex, TriangleList>> {
    let Self {
      width,
      height,
      depth,
      width_segment,
      height_segment,
      depth_segment,
    } = *self;

    let mut indices = vec![];
    let mut vertices = vec![];
    let mut ranges = GeometryRangesInfo::new();

    // helper variables
    let mut number_of_vertices = 0;
    let mut group_start = 0;

    let mut build_plane =
      |u, v, w, u_dir, v_dir, _width, _height, _depth, grid_x, grid_y| {
        let segment_width = _width / grid_x as f32;
        let segment_height = _height / grid_y as f32;
        let width_half = _width / 2.;
        let height_half = _height / 2.;
        let depth_half = _depth / 2.;
        let grid_x1 = grid_x + 1;
        let grid_y1 = grid_y + 1;
        let mut vertex_counter = 0;
        let mut group_count = 0;
        let mut vector = Vec3::splat(0.0);

        // generate vertices, normals and uvs
        for iy in 0..grid_y1 {
          let y = iy as f32 * segment_height - height_half;
          for ix in 0..grid_x1 {
            let x = ix as f32 * segment_width - width_half;
            // set values to correct vector component
            vector[u] = x * u_dir as f32;
            vector[v] = y * v_dir as f32;
            vector[w] = depth_half;
            let position = vector;

            vector[u] = 0.;
            vector[v] = 0.;
            vector[w] = if _depth > 0. { 1. } else { -1. };
            let normal = vector;

            let uv = Vec2::new(ix as f32 / grid_x as f32, 1. - (iy as f32 / grid_y as f32));
            vertices.push(Vertex {
              position,
              normal,
              uv,
            });

            // counters
            vertex_counter += 1;
          }
        }

        // indices
        // 1. you need three indices to draw a single face
        // 2. a single segment consists of two faces
        // 3. so we need to generate six (2*3) indices per segment
        for iy in 0..grid_y {
          for ix in 0..grid_x {
            let a = number_of_vertices + ix + grid_x1 * iy;
            let b = number_of_vertices + ix + grid_x1 * (iy + 1);
            let c = number_of_vertices + (ix + 1) + grid_x1 * (iy + 1);
            let d = number_of_vertices + (ix + 1) + grid_x1 * iy;

            // faces
            indices.push(a as u16);
            indices.push(b as u16);
            indices.push(d as u16);

            indices.push(b as u16);
            indices.push(c as u16);
            indices.push(d as u16);
            // increase counter
            group_count += 6;
          }
        }

        // add a group to the geometry. this will ensure multi material support
        ranges.push(group_start, group_count);
        // calculate new start value for groups
        group_start += group_count;
        // update total number of vertices
        number_of_vertices += vertex_counter;
      };

    // build each side of the box geometrys
    build_plane(2, 1, 0, - 1, - 1, depth, height, width, depth_segment, height_segment); // px
    build_plane(2, 1, 0, 1, - 1, depth, height, - width, depth_segment, height_segment); // nx
    build_plane(0, 2, 1, 1, 1, width, depth, height, width_segment, depth_segment); // py
    build_plane(0, 2, 1, 1, - 1, width, depth, - height, width_segment, depth_segment); // ny
    build_plane(0, 1, 2, 1, - 1, width, height, depth, width_segment, height_segment); // pz
    build_plane(0, 1, 2, - 1, - 1, width, height, - depth, width_segment, height_segment); // nz

    TesselationResult::new(IndexedGeometry::new(vertices, indices), ranges)
  }
}
