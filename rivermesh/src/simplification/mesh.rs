use super::qem::QEM;
use crate::half_edge_mesh::{EdgePairFinder, HalfEdge, HalfEdgeFace, HalfEdgeMesh, HalfEdgeVertex};
use rendiation_math::Vec3;
use rendiation_math_entity::{Face3, Plane};

pub(super) type Mesh = HalfEdgeMesh<VertexData, (), ()>;
pub(super) type Vertex = HalfEdgeVertex<VertexData, (), ()>;
pub(super) type HEdge = HalfEdge<VertexData, (), ()>;
pub(super) type Face = HalfEdgeFace<VertexData, (), ()>;

impl From<&Face> for Face3 {
  fn from(face: &Face) -> Self {
    unsafe {
      let edge_a = face.edge();
      let vert_a = (*edge_a.vert().vertex_data.get()).positions;
      let edge_b = edge_a.next();
      let vert_b = (*edge_b.vert().vertex_data.get()).positions;
      let edge_c = edge_b.next();
      let vert_c = (*edge_c.vert().vertex_data.get()).positions;
      Face3::new(vert_a, vert_b, vert_c)
    }
  }
}

pub struct VertexData {
  pub positions: Vec3<f32>,
  pub normal: Vec3<f32>,
  pub qem: QEM,
}

impl Mesh {
  pub fn from_buffer(positions: &Vec<f32>, indices: &Vec<u32>) -> Self {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let mut edges = Vec::new();

    let mut edge_pairs = EdgePairFinder::new();

    for v in 0..positions.len() / 3 {
      let vert = HalfEdgeVertex::new(
        VertexData {
          positions: Vec3::new(positions[3 * v], positions[3 * v + 1], positions[3 * v + 2]),
          normal: Vec3::new(1.0, 0.0, 0.0),
          qem: QEM::zero(),
        },
        vertices.len(),
      );
      let vert = Box::into_raw(Box::new(vert));
      vertices.push(vert);
    }

    for f in 0..indices.len() / 3 {
      let face = HalfEdgeFace::new_tri(
        vertices[indices[3 * f] as usize],
        vertices[indices[3 * f + 1] as usize],
        vertices[indices[3 * f + 2] as usize],
        &mut edges,
        &mut edge_pairs,
        vertices.len(),
      );
      faces.push(Box::into_raw(Box::new(face)));
    }

    edge_pairs.find_edge_pairs(&mut edges);

    Self {
      edges,
      faces,
      vertices,
    }
  }

  pub fn compute_all_vertices_QEM(&mut self) {
    self.foreach_vertex(|v| {
      let mut vert_qem = QEM::zero();
      v.foreach_surrounding_face(|f| {
        let face3 = Face3::from(f);
        let plane = Plane::from(face3);
        let face_qem = QEM::from(plane);
        vert_qem = vert_qem + face_qem;
      });
      let mut vertex_data = unsafe {&mut *v.vertex_data.get() };
      vertex_data.qem = vert_qem;
    })
  }
}
