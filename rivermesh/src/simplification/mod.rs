use mesh::{HEdge, Mesh, Vertex};
use rendiation_math::Vec3;
use std::{
  cmp::Ordering,
  collections::{BTreeMap, BTreeSet},
};

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

pub struct SimplificationCtx {
  mesh: Mesh,
  qem_edge: BTreeMap<*mut HEdge, OptionEdge>,
  pub target_face_count: usize,
}

impl SimplificationCtx {
  pub fn new(positions: &Vec<f32>, indices: &Vec<u32>) -> Self {
    let mut mesh = Mesh::from_buffer(positions, indices);
    mesh.computeAllVerticesQEM();
    let mut ctx = Self {
      mesh,
      qem_edge: BTreeMap::new(),
      target_face_count: 1000,
    };
    ctx.computeOptionEdges();
    ctx
  }

  fn computeOptionEdges(&mut self) {
    

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
