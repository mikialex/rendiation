use arena::Handle;

use crate::{HalfEdgeFace, HalfEdgeMesh, HalfEdgeMeshData};

use super::HalfEdge;

#[derive(Clone, Copy)]
pub struct HalfEdgeVertex<M: HalfEdgeMeshData> {
  pub data: M::Vertex,
  /// one of the half-edges emanating from the vertex
  pub(super) edge: Handle<HalfEdge<M>>,
}

pub struct HalfEdgeVertexHalfEdgeIter<'a, M: HalfEdgeMeshData> {
  mesh: &'a HalfEdgeMesh<M>,
  has_meet_one_side_boundary: bool,
  has_visited_start: bool,
  start: &'a HalfEdge<M>,
  start_vert: &'a HalfEdgeVertex<M>,
  current: &'a HalfEdge<M>,
}

impl<'a, M: HalfEdgeMeshData> HalfEdgeVertexHalfEdgeIter<'a, M> {
  pub fn next_right(&mut self) -> Option<Handle<HalfEdge<M>>> {
    let current_vert = self.mesh.vertices.get(self.current.vert).unwrap();
    if current_vert as *const _ == self.start_vert as *const _ {
      self.current.pair()
    } else {
      Some(self.current.next())
    }
  }
  pub fn next_left(&mut self) -> Option<Handle<HalfEdge<M>>> {
    let current_vert = self.mesh.vertices.get(self.current.vert).unwrap();
    if current_vert as *const _ == self.start_vert as *const _ {
      self.current.pair()
    } else {
      Some(self.current.prev())
    }
  }
}

impl<'a, M: HalfEdgeMeshData> Iterator for HalfEdgeVertexHalfEdgeIter<'a, M> {
  type Item = &'a HalfEdge<M>;

  fn next(&mut self) -> Option<Self::Item> {
    if !self.has_visited_start {
      self.has_visited_start = true;
      return Some(self.start);
    }

    if !self.has_meet_one_side_boundary {
      if let Some(next) = self.next_right() {
        let next = self.mesh.half_edges.get(next).unwrap();
        if next as *const _ == self.start as *const _ {
          None
        } else {
          Some(next)
        }
      } else {
        self.has_meet_one_side_boundary = true;
        self.current = self.start;
        self
          .next_left()
          .map(|p| self.mesh.half_edges.get(p).unwrap())
      }
    } else {
      self
        .next_left()
        .map(|p| self.mesh.half_edges.get(p).unwrap())
    }
  }
}

pub struct HalfEdgeVertexHalfFaceIter<'a, M: HalfEdgeMeshData> {
  inner: HalfEdgeVertexHalfEdgeIter<'a, M>,
  has_visited_start: bool,
}

impl<'a, M: HalfEdgeMeshData> Iterator for HalfEdgeVertexHalfFaceIter<'a, M> {
  type Item = &'a HalfEdgeFace<M>;

  fn next(&mut self) -> Option<Self::Item> {
    if !self.has_visited_start {
      self.has_visited_start = true;
      let face = self.inner.start.face();
      return Some(self.inner.mesh.faces.get(face).unwrap());
    }

    self
      .inner
      .next()
      .and_then(|_| self.inner.next())
      .map(|edge| self.inner.mesh.faces.get(edge.face).unwrap())
  }
}

impl<M: HalfEdgeMeshData> HalfEdgeVertex<M> {
  pub fn edge(&self) -> Handle<HalfEdge<M>> {
    self.edge
  }

  pub fn is_boundary_vertex(&self, mesh: &HalfEdgeMesh<M>) -> bool {
    self.iter_half_edge(mesh).find(|&e| e.is_border()).is_some()
  }

  pub fn iter_half_edge<'a>(
    &'a self,
    mesh: &'a HalfEdgeMesh<M>,
  ) -> HalfEdgeVertexHalfEdgeIter<'a, M> {
    let start = mesh.half_edges.get(self.edge).unwrap();
    HalfEdgeVertexHalfEdgeIter {
      mesh,
      has_meet_one_side_boundary: false,
      has_visited_start: false,
      start,
      start_vert: self,
      current: start,
    }
  }

  pub fn iter_face<'a>(&'a self, mesh: &'a HalfEdgeMesh<M>) -> HalfEdgeVertexHalfFaceIter<'a, M> {
    HalfEdgeVertexHalfFaceIter {
      inner: self.iter_half_edge(mesh),
      has_visited_start: false,
    }
  }
}
