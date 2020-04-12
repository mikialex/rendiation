use mesh::{Mesh, Vertex};
use rendiation_math::Vec3;
use std::{cmp::Ordering, collections::BTreeSet};

pub mod mesh;
pub mod qem;

struct OptionEdge {
  vertexA: *mut Vertex,
  vertexB: *mut Vertex,
  error: f32,
  new_merge_vertex_position: Vec3<f32>,
}

impl OptionEdge {
  pub fn compute(vertexA: &Vertex, vertexB: &Vertex) -> Self {
    todo!()
  }
}

// impl PartialEq for OptionEdge {
//     fn eq(&self, other: &Self) -> bool {
//         self.error == other.error
//     }
// }

// impl PartialOrd for OptionEdge {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         self.error.partial_cmp(&other.error)
//     }
// }

// impl Ord for OptionEdge{}

pub struct SimplificationCtx {
  mesh: Mesh,
  //   pub qem_edge: BTreeSet<OptionEdge>,
  pub target_face_count: usize,
}

impl SimplificationCtx {
  pub fn new(positions: &Vec<f32>, indices: &Vec<u32>) -> Self {
    let mut mesh = Mesh::from_buffer(positions, indices);
    mesh.computeAllVerticesQEM();
    Self {
      mesh,
      //   qem_edge: BTreeSet::new(),
      target_face_count: 1000,
    }
  }

  fn decimate_edge(&mut self) {
    // remove a edge in mesh
  }

  fn simplify(&mut self) {
    while self.mesh.face_count() > self.target_face_count {
      self.decimate_edge()
    }
  }
}
