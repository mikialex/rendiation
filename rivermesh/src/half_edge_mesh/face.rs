use super::{EdgePairFinder, HalfEdge, HalfEdgeVertex};

pub struct HalfEdgeFace<V, HE, F> {
  id: usize,
  edge: *mut HalfEdge<V, HE, F>, // one of the half-edges bordering the face
}

impl<V, HE, F> HalfEdgeFace<V, HE, F> {
  pub fn new_tri(
    v1: *mut HalfEdgeVertex<V, HE, F>,
    v2: *mut HalfEdgeVertex<V, HE, F>,
    v3: *mut HalfEdgeVertex<V, HE, F>,
    edges: &mut Vec<*mut HalfEdge<V, HE, F>>,
    edge_pairs: &mut EdgePairFinder<V, HE, F>,
    id: usize,
  ) -> Self {
    edges.push(Box::into_raw(Box::new(HalfEdge::new(v1, v2, edges.len()))));
    let edge_v1_v2 = *edges.last_mut().unwrap();
    edges.push(Box::into_raw(Box::new(HalfEdge::new(v2, v3, edges.len()))));
    let edge_v2_v3 = *edges.last_mut().unwrap();
    edges.push(Box::into_raw(Box::new(HalfEdge::new(v3, v1, edges.len()))));
    let edge_v3_v1 = *edges.last_mut().unwrap();

    edge_pairs.insert((v1, v2), edge_v1_v2);
    edge_pairs.insert((v2, v3), edge_v2_v3);
    edge_pairs.insert((v3, v1), edge_v3_v1);

    let mut face = HalfEdgeFace {
      id,
      edge: edge_v1_v2,
    };

    unsafe {
      (*edge_v1_v2).connect_next_edge_for_face(edge_v2_v3, &mut face);
      (*edge_v2_v3).connect_next_edge_for_face(edge_v3_v1, &mut face);
      (*edge_v3_v1).connect_next_edge_for_face(edge_v1_v2, &mut face);
    }
    face
  }

  pub fn id(&self) -> usize {
    self.id
  }

  pub unsafe fn edge_mut(&mut self) -> Option<&mut HalfEdge<V, HE, F>> {
    if self.edge.is_null() {
      None
    } else {
      Some(&mut *self.edge)
    }
  }

  pub fn visit_around_edge_mut(&mut self, mut visitor: impl FnMut(&mut HalfEdge<V, HE, F>)) {
    unsafe {
      if let Some(edge) = self.edge_mut() {
        visitor(edge);
        let edge_ptr = edge as *const HalfEdge<V, HE, F>;
        loop {
          let next_edge = edge.next_mut();
          if next_edge as *const HalfEdge<V, HE, F> != edge_ptr {
            visitor(next_edge);
          } else {
            break;
          }
        }
      }
    }
  }
}
