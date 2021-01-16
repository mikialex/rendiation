pub mod builder;
pub mod edge;
pub mod face;
pub mod vertex;

pub use builder::*;
pub use edge::*;
pub use face::*;
pub use vertex::*;

use arena::*;

pub trait HalfEdgeMeshData {
  type Face: Default;
  type HalfEdge: Default;
  type Vertex;
}

pub struct HalfEdgeMesh<M: HalfEdgeMeshData> {
  half_edges: Arena<HalfEdge<M>>,
  faces: Arena<HalfEdgeFace<M>>,
  vertices: Arena<HalfEdgeVertex<M>>,
}

impl<M: HalfEdgeMeshData> HalfEdgeMesh<M> {
  pub fn new() -> Self {
    Self {
      half_edges: Arena::new(),
      faces: Arena::new(),
      vertices: Arena::new(),
    }
  }

  pub fn face_count(&self) -> usize {
    self.faces.len()
  }

  // pub fn iter_vertex(&mut self) -> impl Iterator<Item = &HalfEdgeVertex<M>> {
  //   self.vertices.iter()
  // }

  // pub fn remove_face(&mut self, face: &mut HalfEdgeFace<M>) {
  //   face.visit_around_edge_mut(|edge| unsafe { self.remove_edge(edge) })
  // }
  // pub unsafe fn remove_edge(&mut self, edge: &mut HalfEdge<M>) {
  //   if let Some(pair) = edge.pair_mut() {
  //     pair.delete_pair();
  //   }
  //   let id = edge.id();
  //   {
  //     let _ = Box::from_raw(*&self.edges[id]);
  //   }
  //   self.edges.swap_remove(id);
  // }
}
