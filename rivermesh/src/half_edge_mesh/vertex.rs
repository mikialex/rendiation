use arena::Handle;

use crate::{HalfEdgeMesh, HalfEdgeMeshData};

use super::{HalfEdge, HalfEdgeFace};

#[derive(Clone, Copy)]
pub struct HalfEdgeVertex<M: HalfEdgeMeshData> {
  pub data: M::Vertex,
  /// one of the half-edges emanating from the vertex
  pub(super) edge: Handle<HalfEdge<M>>,
}

impl<M: HalfEdgeMeshData> HalfEdgeVertex<M> {
  // pub fn new(vertex_data: V) -> HalfEdgeVertex<M> {
  //   HalfEdgeVertex {
  //     vertex_data,
  //     edge: std::ptr::null_mut(),
  //   }
  // }

  pub fn edge(&self) -> Handle<HalfEdge<M>> {
    self.edge
  }

  pub fn is_boundary_vertex(&self, mesh: &HalfEdgeMesh<M>) -> bool {
    todo!()
  }

  // pub fn foreach_surrounding_face(&self, mut visitor: impl FnMut(&HalfEdgeFace<M>)) {
  //   unsafe {
  //     let mut edge = self.edge();
  //     let face = edge.face_mut();
  //     visitor(face);

  //     let mut has_around = false;
  //     while let Some(pair) = edge.pair() {
  //       let face = pair.face_mut();
  //       visitor(face);
  //       let next_edge = pair.next();
  //       if next_edge as *const HalfEdge<M> != edge as *const HalfEdge<M> {
  //         edge = next_edge
  //       } else {
  //         has_around = true;
  //         break;
  //       }
  //     }

  //     if has_around {
  //       return;
  //     }

  //     let mut edge_prev = edge.prev();

  //     while let Some(pair) = edge_prev.pair_mut() {
  //       let face = pair.face_mut();
  //       visitor(face);
  //       edge_prev = pair.prev();
  //     }
  //   }
  // }

  // pub fn visit_surrounding_half_edge_mut(&self, mut visitor: impl FnMut(&HalfEdge<M>)) {
  //   unsafe {
  //     let mut edge = self.edge_mut();
  //     visitor(edge);

  //     let mut has_around = false;
  //     while let Some(pair) = edge.pair_mut() {
  //       visitor(pair);
  //       let next_edge = pair.next_mut();
  //       if next_edge as *const HalfEdge<M> != edge as *const HalfEdge<M> {
  //         visitor(next_edge);
  //         edge = next_edge;
  //       } else {
  //         has_around = true;
  //         break;
  //       }
  //     }

  //     if has_around {
  //       return;
  //     }

  //     let mut edge_prev = edge.prev_mut();
  //     visitor(edge_prev);

  //     while let Some(pair) = edge_prev.pair_mut() {
  //       visitor(pair);
  //       let prev_edge = pair.prev_mut();
  //       visitor(prev_edge);
  //       edge_prev = prev_edge;
  //     }
  //   }
  // }
}
