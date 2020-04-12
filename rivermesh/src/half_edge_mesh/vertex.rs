use super::{HalfEdge, HalfEdgeFace};
use std::cell::UnsafeCell;

pub struct HalfEdgeVertex<V, HE, F> {
  id: usize,
  pub vertex_data: UnsafeCell<V>,
  pub(super) edge: *mut HalfEdge<V, HE, F>, // one of the half-edges emanating from the vertex
}

impl<V, HE, F> HalfEdgeVertex<V, HE, F> {
  pub fn new(vertex_data: V, id: usize) -> HalfEdgeVertex<V, HE, F> {
    HalfEdgeVertex {
      id,
      vertex_data: UnsafeCell::new(vertex_data),
      edge: std::ptr::null_mut(),
    }
  }

  pub fn id(&self) -> usize {
    self.id
  }

  pub unsafe fn edge(&self) -> &HalfEdge<V, HE, F> {
    &*self.edge
  }

  pub unsafe fn edge_mut(&self) -> &mut HalfEdge<V, HE, F> {
    &mut *self.edge
  }

  pub fn foreach_surrounding_face(&self, mut visitor: impl FnMut(&HalfEdgeFace<V, HE, F>)) {
    unsafe {
      let mut edge = self.edge();
      let face = edge.face_mut();
      visitor(face);

      let mut has_around = false;
      while let Some(pair) = edge.pair() {
        let face = pair.face_mut();
        visitor(face);
        let next_edge = pair.next();
        if next_edge as *const HalfEdge<V, HE, F> != edge as *const HalfEdge<V, HE, F> {
          edge = next_edge
        } else {
          has_around = true;
          break;
        }
      }

      if has_around {
        return;
      }

      let mut edge_prev = edge.prev();

      while let Some(pair) = edge_prev.pair_mut() {
        let face = pair.face_mut();
        visitor(face);
        edge_prev = pair.prev();
      }
    }
  }

  pub fn visit_surrounding_half_edge_mut(&self, mut visitor: impl FnMut(&HalfEdge<V, HE, F>)) {
    unsafe {
      let mut edge = self.edge_mut();
      visitor(edge);

      let mut has_around = false;
      while let Some(pair) = edge.pair_mut() {
        visitor(pair);
        let next_edge = pair.next_mut();
        if next_edge as *const HalfEdge<V, HE, F> != edge as *const HalfEdge<V, HE, F> {
          visitor(next_edge);
          edge = next_edge;
        } else {
          has_around = true;
          break;
        }
      }

      if has_around {
        return;
      }

      let mut edge_prev = edge.prev_mut();
      visitor(edge_prev);

      while let Some(pair) = edge_prev.pair_mut() {
        visitor(pair);
        let prev_edge = pair.prev_mut();
        visitor(prev_edge);
        edge_prev = prev_edge;
      }
    }
  }
}
