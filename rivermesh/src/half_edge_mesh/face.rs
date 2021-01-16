use arena::Handle;

use crate::HalfEdgeMeshData;

use super::{HalfEdge, HalfEdgeVertex};

#[derive(Clone, Copy)]
pub struct HalfEdgeFace<M: HalfEdgeMeshData> {
  pub(super) data: M::Face,
  pub(super) edge: Handle<HalfEdge<M>>, // one of the half-edges bordering the face
}

impl<M: HalfEdgeMeshData> HalfEdgeFace<M> {
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
