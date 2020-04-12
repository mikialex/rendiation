use crate::half_edge_mesh::{HalfEdgeMesh, EdgePairFinder, HalfEdgeVertex, HalfEdgeFace, HalfEdge};
use rendiation_math::Vec3;
use super::qem::QEM;
use rendiation_math_entity::Face3;

pub(super) type Mesh = HalfEdgeMesh<VertexData, (), ()>;
pub(super) type Vertex = HalfEdgeVertex<VertexData, (), ()>;
pub(super) type HEdge = HalfEdge<VertexData, (), ()>;
pub(super) type Face = HalfEdgeFace<VertexData, (), ()>;

// impl From<Face> for Face3 {
//   fn from(face: Face) -> Self {

//   }
// }

// fn to_face3

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

  pub fn computeAllVerticesQEM(&mut self){
    self.foreach_vertex_mut(|v|{
      v.foreach_surrounding_face(|f|{
        
      })
    })
  }
}
