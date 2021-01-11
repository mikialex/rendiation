use super::{HalfEdgeFace, HalfEdgeVertex};

// http://www.flipcode.com/archives/The_Half-Edge_Data_Structure.shtml
pub struct HalfEdge<V, HE, F> {
  data: HE,

  /// vertex at the start of the half-edge
  vert: *mut HalfEdgeVertex<V, HE, F>,

  /// oppositely oriented adjacent half-edge
  pair: *mut HalfEdge<V, HE, F>,

  /// face the half-edge borders
  face: *mut HalfEdgeFace<V, HE, F>,

  /// next half-edge around the face
  next: *mut HalfEdge<V, HE, F>,
}

impl<V, HE, F> HalfEdge<V, HE, F> {
  pub(super) fn new(
    from: *mut HalfEdgeVertex<V, HE, F>,
    _to: *mut HalfEdgeVertex<V, HE, F>,
  ) -> HalfEdge<V, HE, F> {
    let mut half_edge = HalfEdge {
      vert: from,
      pair: std::ptr::null_mut(),
      face: std::ptr::null_mut(),
      next: std::ptr::null_mut(),
    };

    // make sure vertex has a edge to point
    unsafe {
      if (*from).edge.is_null() {
        (*from).edge = &mut half_edge
      };
    }

    half_edge
  }

  pub(super) fn connect_next_edge_for_face(
    &mut self,
    next: *mut Self,
    face: &mut HalfEdgeFace<V, HE, F>,
  ) -> &mut Self {
    self.next = next;
    self.face = face;
    self
  }

  pub unsafe fn vert(&self) -> &HalfEdgeVertex<V, HE, F> {
    &*self.vert
  }

  pub unsafe fn vert_mut(&self) -> &mut HalfEdgeVertex<V, HE, F> {
    &mut *self.vert
  }

  pub unsafe fn next(&self) -> &Self {
    &*self.next
  }

  pub unsafe fn next_mut(&self) -> &mut Self {
    &mut *self.next
  }

  pub unsafe fn prev(&self) -> &Self {
    self.next().next()
  }

  pub unsafe fn prev_mut(&self) -> &mut Self {
    self.next_mut().next_mut()
  }

  pub unsafe fn face(&self) -> &HalfEdgeFace<V, HE, F> {
    &*self.face
  }

  pub unsafe fn face_mut(&self) -> &mut HalfEdgeFace<V, HE, F> {
    &mut *self.face
  }

  pub unsafe fn pair_mut(&self) -> Option<&mut Self> {
    if self.pair.is_null() {
      None
    } else {
      Some(&mut *self.pair)
    }
  }

  pub unsafe fn pair(&self) -> Option<&Self> {
    if self.pair.is_null() {
      None
    } else {
      Some(&*self.pair)
    }
  }

  pub unsafe fn delete_pair(&mut self) {
    self.pair = std::ptr::null_mut()
  }

  pub fn is_border(&self) -> bool {
    self.pair.is_null()
  }
}
