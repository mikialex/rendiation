use mesh::Mesh;

pub mod mesh;
pub mod qem;

pub struct SimplificationCtx {
  mesh: Mesh,
  pub target_face_count: usize,
}

impl SimplificationCtx {
  pub fn new(positions: &Vec<f32>, indices: &Vec<u32>) -> Self {
    let mut mesh= Mesh::from_buffer(positions, indices);
    mesh.computeAllVerticesQEM();
    Self {
      mesh,
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
