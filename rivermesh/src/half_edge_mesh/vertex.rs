use super::HalfEdge;

pub struct HalfEdgeVertex<V, HE, F> {
  id: usize,
  pub vertex_data: V,
  pub(super) edge: *mut HalfEdge<V, HE, F>, // one of the half-edges emanating from the vertex
}

impl<V, HE, F> HalfEdgeVertex<V, HE, F> {
  pub fn new(vertex_data: V, id: usize) -> HalfEdgeVertex<V, HE, F> {
    HalfEdgeVertex {
      id,
      vertex_data,
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

  pub fn visit_surrounding_half_edge(&self, visitor: impl Fn(&HalfEdge<V, HE, F>)) {
    unsafe {
      let edge = self.edge_mut();
      visitor(edge);

      let mut no_border_meet = false;
      while let Some(pair) = edge.pair() {
        visitor(pair);
        let next_edge = pair.next();
        if next_edge as *const HalfEdge<V, HE, F> != edge as *const HalfEdge<V, HE, F> {
          visitor(next_edge);
        } else {
          no_border_meet = true;
          break;
        }
      }

      if no_border_meet {
        return;
      }

      let edge_prev = edge.prev();
      visitor(edge_prev);

      while let Some(pair) = edge_prev.pair() {
        visitor(pair);
        let prev_edge = pair.prev();
        visitor(prev_edge);
      }
    }
  }

}
