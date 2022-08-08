use rendiation_algebra::*;

use crate::{
  group::MeshGroupsInfo,
  mesh::{IndexedMesh, TriangleList},
  vertex::Vertex,
};

use super::{GroupedMesh, IndexedMeshTessellator};

#[derive(Copy, Clone, Debug)]
pub struct CylinderMeshParameter {
  pub radius_top: f32,
  pub radius_bottom: f32,
  pub height: f32,
  pub radial_segments: usize,
  pub height_segments: usize,
  pub open_ended: bool,
  pub theta_start: f32,
  pub theta_length: f32,
}

impl Default for CylinderMeshParameter {
  fn default() -> Self {
    Self {
      radius_top: 1.,
      radius_bottom: 1.,
      height: 1.,
      radial_segments: 10,
      height_segments: 1,
      open_ended: false,
      theta_start: 0.,
      theta_length: 2. * f32::PI(),
    }
  }
}

struct CylinderMeshBuilder {
  parameter: CylinderMeshParameter,
  index: usize,
  index_array: Vec<Vec<usize>>,
  group_start: usize,
  indices: Vec<u16>,
  vertices: Vec<Vertex>,
  ranges: MeshGroupsInfo,
}

impl CylinderMeshBuilder {
  fn new(parameter: CylinderMeshParameter) -> Self {
    Self {
      parameter,
      indices: vec![],
      vertices: vec![],
      ranges: MeshGroupsInfo::new(),

      // helper letiables
      index: 0,
      index_array: vec![],
      group_start: 0,
    }
  }

  fn generate_torso(&mut self) {
    let CylinderMeshParameter {
      radius_top,
      radius_bottom,
      height,
      radial_segments,
      height_segments,
      theta_start,
      theta_length,
      ..
    } = self.parameter;
    let mut group_count = 0;

    // this will be used to calculate the normal
    let slope = (radius_bottom - radius_top) / height;

    // generate vertices, normals and uvs

    for y in 0..=height_segments {
      let mut index_row = vec![];

      let v = y as f32 / height_segments as f32;

      // calculate the radius of the current row

      let radius = v * (radius_bottom - radius_top) + radius_top;

      for x in 0..=radial_segments {
        let u = x as f32 / radial_segments as f32;

        let theta = u * theta_length + theta_start;

        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        self.vertices.push(Vertex {
          position: Vec3::new(
            radius * sin_theta,
            -v * height + height / 2.,
            radius * cos_theta,
          ),
          normal: Vec3::new(sin_theta, slope, cos_theta).normalize(),
          uv: Vec2::new(u, 1. - v),
        });

        // save index of vertex in respective row
        self.index += 1;
        index_row.push(self.index);
      }

      // now save vertices of the row in our index array

      self.index_array.push(index_row);
    }

    // generate indices
    for x in 0..radial_segments {
      for y in 0..height_segments {
        // we use the index array to access the correct indices
        let a = self.index_array[y][x];
        let b = self.index_array[y + 1][x];
        let c = self.index_array[y + 1][x + 1];
        let d = self.index_array[y][x + 1];

        // faces
        self.indices.push(a as u16);
        self.indices.push(b as u16);
        self.indices.push(d as u16);

        self.indices.push(b as u16);
        self.indices.push(c as u16);
        self.indices.push(d as u16);

        // update group counter
        group_count += 6;
      }
    }

    // add a group to the mesh. this will ensure multi material support
    self.ranges.push(self.group_start, group_count);

    // calculate new start value for groups

    self.group_start += group_count;
  }

  fn generate_cap(&mut self, top: bool) {
    let CylinderMeshParameter {
      radius_top,
      radius_bottom,
      height,
      radial_segments,
      theta_start,
      theta_length,
      ..
    } = self.parameter;

    let mut group_count = 0;

    let radius = if top { radius_top } else { radius_bottom };
    let sign = if top { 1 } else { -1 };

    // save the index of the first center vertex
    let center_index_start = self.index;

    // first we generate the center vertex data of the cap.
    // because the mesh needs one set of uvs per face,
    // we must generate a center vertex per face/segment
    for _ in 1..=radial_segments {
      self.vertices.push(Vertex {
        position: Vec3::new(0., height / 2. * sign as f32, 0.),
        normal: Vec3::new(0., sign as f32, 0.),
        uv: Vec2::new(0.5, 0.5),
      });
      self.index += 1;
    }

    // save the index of the last center vertex

    let center_index_end = self.index;

    // now we generate the surrounding vertices, normals and uvs

    for x in 0..=radial_segments {
      let u = x as f32 / radial_segments as f32;
      let theta = u * theta_length + theta_start;

      let cos_theta = theta.cos();
      let sin_theta = theta.sin();

      self.vertices.push(Vertex {
        position: Vec3::new(
          radius * sin_theta,
          height / 2. * sign as f32,
          radius * cos_theta,
        ),
        normal: Vec3::new(0., sign as f32, 0.),
        uv: Vec2::new(cos_theta * 0.5 + 0.5, sin_theta * 0.5 * sign as f32 + 0.5),
      });
      self.index += 1;
    }

    // generate indices
    for x in 0..radial_segments {
      let c = center_index_start + x;
      let i = center_index_end + x;

      #[allow(clippy::branches_sharing_code)]
      if top {
        self.indices.push(i as u16);
        self.indices.push((i + 1) as u16);
        self.indices.push(c as u16);
      } else {
        self.indices.push((i + 1) as u16);
        self.indices.push(i as u16);
        self.indices.push(c as u16);
      }

      group_count += 3;
    }

    // add a group to the mesh. this will ensure multi material support
    self.ranges.push(self.group_start, group_count);

    // calculate new start value for groups

    self.group_start += group_count;
  }
}

impl IndexedMeshTessellator for CylinderMeshParameter {
  fn tessellate(&self) -> GroupedMesh<IndexedMesh<TriangleList, Vec<Vertex>, Vec<u16>>> {
    let mut builder = CylinderMeshBuilder::new(*self);

    // generate mesh
    builder.generate_torso();

    if !self.open_ended {
      if self.radius_top > 0. {
        builder.generate_cap(true)
      };
      if self.radius_bottom > 0. {
        builder.generate_cap(false)
      };
    }

    GroupedMesh::new(
      IndexedMesh::new(builder.vertices, builder.indices),
      builder.ranges,
    )
  }
}
