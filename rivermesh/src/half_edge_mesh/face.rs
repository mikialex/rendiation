use super::{EdgePairFinder, HalfEdge, HalfEdgeVertex};

pub struct HalfEdgeFace<V, HE, F> {
  edge: *mut HalfEdge<V, HE, F>, // one of the half-edges bordering the face
}

impl<V, HE, F> HalfEdgeFace<V, HE, F> {
  pub fn new_tri(
    v1: *mut HalfEdgeVertex<V, HE, F>,
    v2: *mut HalfEdgeVertex<V, HE, F>,
    v3: *mut HalfEdgeVertex<V, HE, F>,
    edges: &mut Vec<*mut HalfEdge<V, HE, F>>,
    edge_pairs: &mut EdgePairFinder<V, HE, F>,
  ) -> Self {
    edges.push(Box::into_raw(Box::new(HalfEdge::new(v1, v2))));
    let edge_v1_v2 = *edges.last_mut().unwrap();
    edges.push(Box::into_raw(Box::new(HalfEdge::new(v2, v3))));
    let edge_v2_v3 = *edges.last_mut().unwrap();
    edges.push(Box::into_raw(Box::new(HalfEdge::new(v3, v1))));
    let edge_v3_v1 = *edges.last_mut().unwrap();

    edge_pairs.insert((v1, v2), edge_v1_v2);
    edge_pairs.insert((v2, v3), edge_v2_v3);
    edge_pairs.insert((v3, v1), edge_v3_v1);

    let mut face = HalfEdgeFace { edge: edge_v1_v2 };

    unsafe {
      (*edge_v1_v2).connect_next_edge_for_face(edge_v2_v3, &mut face);
      (*edge_v2_v3).connect_next_edge_for_face(edge_v3_v1, &mut face);
      (*edge_v3_v1).connect_next_edge_for_face(edge_v1_v2, &mut face);
    }
    face
  }

  pub unsafe fn edge_mut(&self) -> &mut HalfEdge<V, HE, F> {
    &mut *self.edge
  }

  pub unsafe fn edge(&self) -> &HalfEdge<V, HE, F> {
    &*self.edge
  }

  pub unsafe fn visit_around_edge(&mut self, mut visitor: impl FnMut(&HalfEdge<V, HE, F>)) {
    let mut edge = self.edge();

    loop {
      visitor(edge);
      let next_edge = edge.next();
      if next_edge as *const HalfEdge<V, HE, F> != edge as *const HalfEdge<V, HE, F> {
        break;
      }
      edge = next_edge;
    }
  }

  pub unsafe fn visit_around_edge_mut(&mut self, mut visitor: impl FnMut(&mut HalfEdge<V, HE, F>)) {
    let mut edge = self.edge_mut();

    loop {
      visitor(edge);
      let next_edge = edge.next_mut();
      if next_edge as *const HalfEdge<V, HE, F> != edge as *const HalfEdge<V, HE, F> {
        break;
      }
      edge = next_edge;
    }
  }
}
