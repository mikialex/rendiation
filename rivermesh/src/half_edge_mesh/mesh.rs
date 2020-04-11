use super::{HalfEdge, HalfEdgeFace, HalfEdgeVertex};
use std::collections::HashMap;

pub struct HalfEdgeMesh<V = (), HE = (), F = ()> {
  pub(crate) edges: Vec<*mut HalfEdge<V, HE, F>>,
  pub(crate) faces: Vec<*mut HalfEdgeFace<V, HE, F>>,
  pub(crate) vertices: Vec<*mut HalfEdgeVertex<V, HE, F>>,
}

impl<V, HE, F> HalfEdgeMesh<V, HE, F> {
  pub fn face_count(&self) -> usize{
    self.faces.len()
  }

  pub fn remove_face(&mut self, face: &mut HalfEdgeFace<V, HE, F>) {
    face.visit_around_edge_mut(|edge| unsafe { self.remove_edge(edge) })
  }
  pub unsafe fn remove_edge(&mut self, edge: &mut HalfEdge<V, HE, F>) {
    if let Some(pair) = edge.pair_mut() {
      pair.delete_pair();
    }
    let id = edge.id();
    {
      let _ = Box::from_raw(*&self.edges[id]);
    }
    self.edges.swap_remove(id);
  }
}

impl<V, HE, F> Drop for HalfEdgeMesh<V, HE, F> {
  fn drop(&mut self) {
    println!("drop");
    for v in &self.vertices {
      unsafe {
        let _ = Box::from_raw(*v);
      }
    }
    for v in &self.faces {
      unsafe {
        let _ = Box::from_raw(*v);
      }
    }
    for v in &self.edges {
      unsafe {
        let _ = Box::from_raw(*v);
      }
    }
  }
}

pub struct EdgePairFinder<V, HE, F>(
  HashMap<(*mut HalfEdgeVertex<V, HE, F>, *mut HalfEdgeVertex<V, HE, F>), *mut HalfEdge<V, HE, F>>,
);

impl<V, HE, F> EdgePairFinder<V, HE, F> {
  pub fn new() -> Self {
    EdgePairFinder(HashMap::new())
  }
  pub fn insert(
    &mut self,
    k: (*mut HalfEdgeVertex<V, HE, F>, *mut HalfEdgeVertex<V, HE, F>),
    v: *mut HalfEdge<V, HE, F>,
  ) {
    if let Some(_) = self.0.insert(k, v) {
      panic!("not support none manifold geometry")
    }
  }

  pub fn find_edge_pairs(&self, edges: &mut Vec<*mut HalfEdge<V, HE, F>>) {
    unsafe {
      for edge in edges {
        let edge = &mut **edge;
        if edge.pair_mut().is_none() {
          let key = (
            edge.next_mut().vert_mut() as *mut HalfEdgeVertex<V, HE, F>,
            edge.vert_mut() as *mut HalfEdgeVertex<V, HE, F>,
          );
          if let Some(pair) = self.0.get(&key) {
            edge.pair = *pair as *mut HalfEdge<V, HE, F>;
          }
        }
      }
    }
  }
}
