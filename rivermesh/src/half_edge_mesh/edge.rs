use arena::Handle;

use super::{HalfEdgeFace, HalfEdgeVertex};

#[derive(Clone, Copy)]
// http://www.flipcode.com/archives/The_Half-Edge_Data_Structure.shtml
pub struct HalfEdge<V, HE, F> {
  pub data: HE,

  /// vertex at the start of the half-edge
  pub(super) vert: Handle<HalfEdgeVertex<V, HE, F>>,

  /// oppositely oriented adjacent half-edge
  pub(super) pair: Option<Handle<HalfEdge<V, HE, F>>>,

  /// face the half-edge borders
  pub(super) face: Handle<HalfEdgeFace<V, HE, F>>,

  /// next half-edge around the face
  pub(super) next: Handle<HalfEdge<V, HE, F>>,
}

impl<V, HE, F> HalfEdge<V, HE, F> {
  // pub(super) fn new(
  //   from: *mut HalfEdgeVertex<V, HE, F>,
  //   _to: *mut HalfEdgeVertex<V, HE, F>,
  // ) -> HalfEdge<V, HE, F> {
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
  //   face: &mut HalfEdgeFace<V, HE, F>,
  // ) -> &mut Self {
  //   self.next = next;
  //   self.face = face;
  //   self
  // }

  pub fn vert(&self) -> Handle<HalfEdgeVertex<V, HE, F>> {
    self.vert
  }

  pub fn next(&self) -> Handle<Self> {
    self.next
  }

  // pub  fn prev(&self) ->  Handle<Self> {
  //   self.next().next()
  // }

  pub unsafe fn face(&self) -> Handle<HalfEdgeFace<V, HE, F>> {
    self.face
  }

  pub unsafe fn pair(&self) -> Option<Handle<Self>> {
    self.pair
  }

  pub fn is_border(&self) -> bool {
    self.pair.is_none()
  }
}
