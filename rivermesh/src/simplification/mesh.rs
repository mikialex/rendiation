use super::qem::QEM;
use crate::{
  half_edge_mesh::{HalfEdge, HalfEdgeFace, HalfEdgeMesh, HalfEdgeVertex},
  HalfEdgeMeshData,
};
use rendiation_math::*;
use rendiation_math_entity::{Plane, Triangle};

pub struct SimplificationMeshData;

impl HalfEdgeMeshData for SimplificationMeshData {
  type Face = ();
  type HalfEdge = EdgeData;
  type Vertex = VertexData;
}

pub(super) type Mesh = HalfEdgeMesh<SimplificationMeshData>;
pub(super) type Vertex = HalfEdgeVertex<SimplificationMeshData>;
pub(super) type HEdge = HalfEdge<SimplificationMeshData>;
pub(super) type Face = HalfEdgeFace<SimplificationMeshData>;

impl From<&Face> for Triangle {
  fn from(face: &Face) -> Self {
    todo!()
    // unsafe {
    //   let edge_a = face.edge();
    //   let vert_a = (*edge_a.vert().vertex_data.get()).positions;
    //   let edge_b = edge_a.next();
    //   let vert_b = (*edge_b.vert().vertex_data.get()).positions;
    //   let edge_c = edge_b.next();
    //   let vert_c = (*edge_c.vert().vertex_data.get()).positions;
    //   Triangle::new(vert_a, vert_b, vert_c)
    // }
  }
}

pub struct VertexData {
  pub positions: Vec3<f32>,
  pub normal: Vec3<f32>,
  pub qem: QEM,
}

pub struct EdgeData {
  pub update_id: u32,
}

impl Default for EdgeData {
  fn default() -> Self {
    Self { update_id: 0 }
  }
}

impl EdgeData {
  pub fn compute(vertex_a: &Vertex, vertex_b: &Vertex) -> Self {
    todo!()
  }
}

impl Mesh {
  pub fn from_buffer(positions: &Vec<f32>, indices: &Vec<u32>) -> Self {
    todo!()
    // let mut vertices = Vec::new();
    // let mut faces = Vec::new();
    // let mut edges = Vec::new();

    // let mut edge_pairs = EdgePairFinder::new();

    // for v in 0..positions.len() / 3 {
    //   let vert = HalfEdgeVertex::new(
    //     VertexData {
    //       positions: Vec3::new(positions[3 * v], positions[3 * v + 1], positions[3 * v + 2]),
    //       normal: Vec3::new(1.0, 0.0, 0.0),
    //       qem: QEM::zero(),
    //     },
    //     vertices.len(),
    //   );
    //   let vert = Box::into_raw(Box::new(vert));
    //   vertices.push(vert);
    // }

    // for f in 0..indices.len() / 3 {
    //   let face = HalfEdgeFace::new_tri(
    //     vertices[indices[3 * f] as usize],
    //     vertices[indices[3 * f + 1] as usize],
    //     vertices[indices[3 * f + 2] as usize],
    //     &mut edges,
    //     &mut edge_pairs,
    //     vertices.len(),
    //   );
    //   faces.push(Box::into_raw(Box::new(face)));
    // }

    // edge_pairs.find_edge_pairs(&mut edges);

    // Self {
    //   edges,
    //   faces,
    //   vertices,
    // }
  }

  pub fn compute_all_vertices_qem(&mut self) {
    // self.foreach_vertex(|v| {
    //   let mut vert_qem = QEM::zero();
    //   v.foreach_surrounding_face(|f| {
    //     let face3 = Triangle::from(f);
    //     let plane = Plane::from(face3);
    //     let face_qem = QEM::from(plane);
    //     vert_qem = vert_qem + face_qem;
    //   });
    //   let mut vertex_data = unsafe { &mut *v.vertex_data.get() };
    //   vertex_data.qem = vert_qem;
    // })
  }
}
