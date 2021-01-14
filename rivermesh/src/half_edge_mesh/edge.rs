use arena::Handle;

use crate::{HalfEdgeMesh, HalfEdgeMeshData};

use super::{HalfEdgeFace, HalfEdgeVertex};

#[derive(Clone, Copy)]
// http://www.flipcode.com/archives/The_Half-Edge_Data_Structure.shtml
pub struct HalfEdge<M: HalfEdgeMeshData> {
  pub data: M::HalfEdge,

  /// vertex at the start of the half-edge
  pub(super) vert: Handle<HalfEdgeVertex<M>>,

  /// oppositely oriented adjacent half-edge
  pub(super) pair: Option<Handle<HalfEdge<M>>>,

  /// face the half-edge borders
  pub(super) face: Handle<HalfEdgeFace<M>>,

  /// next half-edge around the face
  pub(super) next: Handle<HalfEdge<M>>,
}

impl<M: HalfEdgeMeshData> HalfEdge<M> {
  pub fn get_by_two_points(
    mesh: &HalfEdgeMesh<M>,
    from: Handle<HalfEdgeVertex<M>>,
    to: Handle<HalfEdgeVertex<M>>,
  ) -> Option<Handle<HalfEdge<M>>> {
    todo!()
  }

  // pub(super) fn new(
  //   from: Handle<HalfEdgeVertex<M>>,
  //   _to: Handle<HalfEdgeVertex<M>>,
  // ) -> HalfEdge<M> {
  //   let mut half_edge = HalfEdge {
  //     vert: from,
  //     pair: std::ptr::null_mut(),
  //     face: std::ptr::null_mut(),
  //     next: std::ptr::null_mut(),
  //   };

  //   // make sure vertex has a edge to point
  //   unsafe {
  //     if (*from).edge.is_null() {
  //       (*from).edge = &mut half_edge
  //     };
  //   }

  //   half_edge
  // }

  // pub(super) fn connect_next_edge_for_face(
  //   &mut self,
  //   next: *mut Self,
  //   face: &mut HalfEdgeFace<M>,
  // ) -> &mut Self {
  //   self.next = next;
  //   self.face = face;
  //   self
  // }

  pub fn vert(&self) -> Handle<HalfEdgeVertex<M>> {
    self.vert
  }

  pub fn next(&self) -> Handle<Self> {
    self.next
  }

  // pub  fn prev(&self) ->  Handle<Self> {
  //   self.next().next()
  // }

  pub unsafe fn face(&self) -> Handle<HalfEdgeFace<M>> {
    self.face
  }

  pub unsafe fn pair(&self) -> Option<Handle<Self>> {
    self.pair
  }

  pub fn is_border(&self) -> bool {
    self.pair.is_none()
  }
}
