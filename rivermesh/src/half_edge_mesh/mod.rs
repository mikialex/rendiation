pub mod builder;
pub mod edge;
pub mod face;
pub mod vertex;

pub use builder::*;
pub use edge::*;
pub use face::*;
pub use vertex::*;

use arena::*;
use std::collections::HashMap;

pub trait HalfEdgeMeshData {
  type Face;
  type HalfEdge;
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

// pub struct EdgePairFinder<M>(
//   HashMap<(*mut HalfEdgeVertex<M>, *mut HalfEdgeVertex<M>), *mut HalfEdge<M>>,
// );

// impl<M> EdgePairFinder<M> {
//   pub fn new() -> Self {
//     EdgePairFinder(HashMap::new())
//   }
//   pub fn insert(
//     &mut self,
//     k: (*mut HalfEdgeVertex<M>, *mut HalfEdgeVertex<M>),
//     v: *mut HalfEdge<M>,
//   ) {
//     if let Some(_) = self.0.insert(k, v) {
//       panic!("not support none manifold geometry")
//     }
//   }

//   pub fn find_edge_pairs(&self, edges: &mut Vec<*mut HalfEdge<M>>) {
//     unsafe {
//       for edge in edges {
//         let edge = &mut **edge;
//         if edge.pair_mut().is_none() {
//           let key = (
//             edge.next_mut().vert_mut() as *mut HalfEdgeVertex<M>,
//             edge.vert_mut() as *mut HalfEdgeVertex<M>,
//           );
//           if let Some(pair) = self.0.get(&key) {
//             edge.pair = *pair as *mut HalfEdge<M>;
//           }
//         }
//       }
//     }
//   }
// }
