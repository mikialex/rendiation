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

  // pub fn visit_around_edge_mut(&self, visitor: &mut dyn FnMut(&mut HalfEdge<V, HE, F>)) {
  //   let edge = self.edge_mut();
  //   visitor(edge);
  //   loop {
  //     if let Some(pair) = edge.pair_mut() {
  //       let next_edge = pair.next_mut();
  //       if next_edge as *const HalfEdge<V, HE, F> != edge as *const HalfEdge<V, HE, F> {
  //         visitor(next_edge);
  //       } else {
  //         break;
  //       }
  //     }
  //   }
  // }
}
