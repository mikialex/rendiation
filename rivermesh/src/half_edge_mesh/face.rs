use arena::Handle;

use crate::HalfEdgeMeshData;

use super::{HalfEdge, HalfEdgeVertex};

#[derive(Clone, Copy)]
pub struct HalfEdgeFace<M: HalfEdgeMeshData> {
  pub(super) data: M::Face,
  pub(super) edge: Handle<HalfEdge<M>>, // one of the half-edges bordering the face
}

impl<M: HalfEdgeMeshData> HalfEdgeFace<M> {
  // pub fn new_tri(
  //   v1: *mut HalfEdgeVertex<M>,
  //   v2: *mut HalfEdgeVertex<M>,
  //   v3: *mut HalfEdgeVertex<M>,
  //   edges: &mut Vec<*mut HalfEdge<M>>,
  //   edge_pairs: &mut EdgePairFinder<M>,
  // ) -> Self {
  //   edges.push(Box::into_raw(Box::new(HalfEdge::new(v1, v2))));
  //   let edge_v1_v2 = *edges.last_mut().unwrap();
  //   edges.push(Box::into_raw(Box::new(HalfEdge::new(v2, v3))));
  //   let edge_v2_v3 = *edges.last_mut().unwrap();
  //   edges.push(Box::into_raw(Box::new(HalfEdge::new(v3, v1))));
  //   let edge_v3_v1 = *edges.last_mut().unwrap();

  //   edge_pairs.insert((v1, v2), edge_v1_v2);
  //   edge_pairs.insert((v2, v3), edge_v2_v3);
  //   edge_pairs.insert((v3, v1), edge_v3_v1);

  //   let mut face = HalfEdgeFace { edge: edge_v1_v2 };

  //   unsafe {
  //     (*edge_v1_v2).connect_next_edge_for_face(edge_v2_v3, &mut face);
  //     (*edge_v2_v3).connect_next_edge_for_face(edge_v3_v1, &mut face);
  //     (*edge_v3_v1).connect_next_edge_for_face(edge_v1_v2, &mut face);
  //   }
  //   face
  // }

  pub fn edge(&self) -> Handle<HalfEdge<M>> {
    self.edge
  }

  // pub unsafe fn visit_around_edge(&mut self, mut visitor: impl FnMut(&HalfEdge<M>)) {
  //   let mut edge = self.edge();

  //   loop {
  //     visitor(edge);
  //     let next_edge = edge.next();
  //     if next_edge as *const HalfEdge<M> != edge as *const HalfEdge<M> {
  //       break;
  //     }
  //     edge = next_edge;
  //   }
  // }

  // pub unsafe fn visit_around_edge_mut(&mut self, mut visitor: impl FnMut(&mut HalfEdge<M>)) {
  //   let mut edge = self.edge_mut();

  //   loop {
  //     visitor(edge);
  //     let next_edge = edge.next_mut();
  //     if next_edge as *const HalfEdge<M> != edge as *const HalfEdge<M> {
  //       break;
  //     }
  //     edge = next_edge;
  //   }
  // }
}
