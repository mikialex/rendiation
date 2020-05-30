use mesh::{HEdge, Mesh, Vertex};
use rendiation_math::Vec3;
use std::{
  cmp::Ordering,
  collections::{BTreeMap, BTreeSet},
};

pub mod mesh;
pub mod qem;

struct OptionEdge {
  vertex_a: *mut Vertex,
  vertex_b: *mut Vertex,
  error: f32,
  new_merge_vertex_position: Vec3<f32>,
}

impl OptionEdge {
  pub fn compute(vertex_a: &Vertex, vertex_b: &Vertex) -> Self {
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
    mesh.compute_all_vertices_qem();
    let mut ctx = Self {
      mesh,
      qem_edge: BTreeMap::new(),
      target_face_count: 1000,
    };
    ctx.compute_option_edges();
    ctx
  }

  fn compute_option_edges(&mut self) {}

  fn decimate_edge(&mut self) {
    // remove a edge in mesh
  }

  fn simplify(&mut self) {
    while self.mesh.face_count() > self.target_face_count {
      self.decimate_edge()
    }
  }
}
