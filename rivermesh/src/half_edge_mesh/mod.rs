pub mod mesh;
pub use mesh::*;

pub struct HalfEdgeVertex<T> {
  id: usize,
  pub vertex_data: T,
  edge: *mut HalfEdge<T>, // one of the half-edges emanating from the vertex
}

impl<T> HalfEdgeVertex<T> {
  pub fn new(vertex_data: T, id: usize) -> HalfEdgeVertex<T> {
    HalfEdgeVertex {
      id,
      vertex_data,
      edge: std::ptr::null_mut(),
    }
  }

  pub fn edge(&self) -> &HalfEdge<T> {
    // it should always valid in valid half edge mesh
    unsafe { &*self.edge }
  }

  pub fn edge_mut(&self) -> &mut HalfEdge<T> {
    // it should always valid in valid half edge mesh
    unsafe { &mut *self.edge }
  }

  pub fn visit_around_edge_mut(&self, visitor: &mut dyn FnMut(&mut HalfEdge<T>)) {
    let edge = self.edge_mut();
    visitor(edge);
    loop {
      if let Some(pair) = edge.pair_mut() {
        let next_edge = pair.next_mut();
        if next_edge as *const HalfEdge<T> != edge as *const HalfEdge<T> {
          visitor(next_edge);
        } else {
          break;
        }
      }
    }
  }
}

pub struct HalfEdgeFace<T> {
  id: usize,
  edge: *mut HalfEdge<T>, // one of the half-edges bordering the face
}

impl<T> HalfEdgeFace<T> {
  pub fn new_tri(
    v1: *mut HalfEdgeVertex<T>,
    v2: *mut HalfEdgeVertex<T>,
    v3: *mut HalfEdgeVertex<T>,
    edges: &mut Vec<*mut HalfEdge<T>>,
    edge_pairs: &mut EdgePairFinder<T>,
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

  pub fn edge_mut(&mut self) -> Option<&mut HalfEdge<T>> {
    if self.edge.is_null() {
      return None;
    }
    unsafe {
      return Some(&mut *self.edge);
    }
  }

  pub fn visit_around_edge_mut(&mut self, visitor: impl Fn(&HalfEdge<T>)) {
    if let Some(edge) = self.edge_mut() {
      visitor(edge);
      let edge_ptr = edge as *const HalfEdge<T>;
      loop {
        let next_edge = edge.next_mut();
        if next_edge as *const HalfEdge<T> != edge_ptr {
          visitor(next_edge);
        } else {
          break;
        }
      }
    }
  }
}

// http://www.flipcode.com/archives/The_Half-Edge_Data_Structure.shtml
pub struct HalfEdge<T> {
  id: usize,

  /// vertex at the start of the half-edge
  vert: *mut HalfEdgeVertex<T>,

  /// oppositely oriented adjacent half-edge
  pair: *mut HalfEdge<T>,

  /// face the half-edge borders
  face: *mut HalfEdgeFace<T>,

  /// next half-edge around the face
  next: *mut HalfEdge<T>,
}

impl<T> HalfEdge<T> {
  fn new(from: *mut HalfEdgeVertex<T>, _to: *mut HalfEdgeVertex<T>, id: usize) -> HalfEdge<T> {
    let mut half_edge = HalfEdge {
      id,
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

  fn connect_next_edge_for_face(
    &mut self,
    next: *mut Self,
    face: &mut HalfEdgeFace<T>,
  ) -> &mut Self {
    self.next = next;
    self.face = face;
    self
  }

  pub fn vert(&self) -> &HalfEdgeVertex<T> {
    unsafe { &*self.vert }
  }

  pub fn vert_mut(&mut self) -> &mut HalfEdgeVertex<T> {
    unsafe { &mut *self.vert }
  }

  pub fn next(&self) -> &Self {
    unsafe { &*self.next }
  }

  pub fn next_mut(&mut self) -> &mut Self {
    unsafe { &mut *self.next }
  }

  pub fn face(&self) -> &HalfEdgeFace<T> {
    unsafe { &*self.face }
  }

  pub fn pair_mut(&self) -> Option<&mut Self> {
    if self.pair.is_null() {
      None
    } else {
      unsafe { Some(&mut *self.pair) }
    }
  }

  pub fn is_border(&self) -> bool {
    self.pair.is_null()
  }
}
