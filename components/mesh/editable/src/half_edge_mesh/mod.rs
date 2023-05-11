pub mod builder;
pub mod edge;
pub mod face;
mod test;
pub mod vertex;

use std::ops::Index;

use arena::*;
pub use builder::*;
pub use edge::*;
pub use face::*;
pub use vertex::*;

pub trait HalfEdgeMeshData {
  type Face: Default;
  type HalfEdge: Default;
  type Vertex;
}

pub struct HalfEdgeMesh<M: HalfEdgeMeshData> {
  pub half_edges: Arena<HalfEdge<M>>, // todo not pub
  pub faces: Arena<HalfEdgeFace<M>>,
  pub vertices: Arena<HalfEdgeVertex<M>>,
}

impl<M: HalfEdgeMeshData> Index<Handle<HalfEdgeVertex<M>>> for HalfEdgeMesh<M> {
  type Output = HalfEdgeVertex<M>;

  fn index(&self, index: Handle<HalfEdgeVertex<M>>) -> &Self::Output {
    &self.vertices[index]
  }
}

impl<M: HalfEdgeMeshData> Index<Handle<HalfEdge<M>>> for HalfEdgeMesh<M> {
  type Output = HalfEdge<M>;

  fn index(&self, index: Handle<HalfEdge<M>>) -> &Self::Output {
    &self.half_edges[index]
  }
}

impl<M: HalfEdgeMeshData> Index<Handle<HalfEdgeFace<M>>> for HalfEdgeMesh<M> {
  type Output = HalfEdgeFace<M>;

  fn index(&self, index: Handle<HalfEdgeFace<M>>) -> &Self::Output {
    &self.faces[index]
  }
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
