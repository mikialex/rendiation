use arena::Handle;
use mesh::{HEdge, Mesh};
use rendiation_math::Vec3;
use rendiation_math_entity::{Plane, Triangle};
use std::collections::{BTreeMap, BinaryHeap};

use crate::HalfEdge;

use self::{mesh::SimplificationMeshData, qem::QEM};

pub mod mesh;
pub mod qem;

pub struct SimplificationCtx {
  mesh: Mesh,
  edge_choices: BinaryHeap<EdgeChoice>,
}

pub enum SimplificationError {
  NotEnoughEdgeForDecimation,
}
use SimplificationError::*;

pub struct EdgeChoice {
  edge: Handle<HalfEdge<SimplificationMeshData>>,
  dirty_id: u32,
  error: f32,
  new_merge_vertex_position: Vec3<f32>,
}

impl PartialOrd for EdgeChoice {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    todo!()
  }
}

impl PartialEq for EdgeChoice {
  fn eq(&self, other: &Self) -> bool {
    todo!()
  }
}

impl Eq for EdgeChoice {}

impl Ord for EdgeChoice {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    todo!()
  }
}

impl SimplificationCtx {
  pub fn new(positions: &Vec<f32>, indices: &Vec<u32>) -> Self {
    let mesh = Mesh::from_buffer(positions, indices);

    // compute_all_vertices_qem
    mesh.vertices.iter().for_each(|(_, v)| {
      let mut vert_qem = QEM::zero();
      v.iter_face(&mesh).for_each(|(f, _)| {
        let face3 = Triangle::from(f);
        let plane = Plane::from(face3);
        let face_qem = QEM::from(plane);
        vert_qem += face_qem;
      });
      v.data.qem.set(vert_qem)
    });

    let mut ctx = Self {
      mesh,
      edge_choices: BinaryHeap::new(),
    };
    ctx.compute_option_edges();
    ctx
  }

  fn compute_option_edges(&mut self) {}

  /// remove a edge in mesh
  fn decimate_edge(&mut self) -> bool {
    while let Some(edge_record) = self.edge_choices.pop() {
      let edge = self.mesh.half_edges.get(edge_record.edge).unwrap();
      if edge.data.update_id.get() != edge_record.dirty_id {
        continue;
      }
      // todo
      return true;
    }
    false
  }

  fn simplify(&mut self, target_face_count: usize) -> Result<(), SimplificationError> {
    while self.mesh.face_count() > target_face_count {
      if !self.decimate_edge() {
        return Err(NotEnoughEdgeForDecimation);
      }
    }
    Ok(())
  }
}
