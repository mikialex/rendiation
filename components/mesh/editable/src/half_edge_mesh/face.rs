use arena::Handle;

use super::HalfEdge;
use crate::{EdgeIterItem, HalfEdgeMesh, HalfEdgeMeshData};

#[derive(Clone, Copy)]
pub struct HalfEdgeFace<M: HalfEdgeMeshData> {
  pub(super) data: M::Face,
  pub(super) edge: Handle<HalfEdge<M>>, // one of the half-edges bordering the face
}

impl<M: HalfEdgeMeshData> HalfEdgeFace<M> {
  pub fn edge(&self) -> Handle<HalfEdge<M>> {
    self.edge
  }

  pub fn iter_half_edge<'a>(&'a self, mesh: &'a HalfEdgeMesh<M>) -> FaceToHalfEdgeIter<'a, M> {
    FaceToHalfEdgeIter {
      mesh,
      start: self.edge,
      last: None,
    }
  }

  pub fn side_count(&self, mesh: &HalfEdgeMesh<M>) -> usize {
    self.iter_half_edge(mesh).count()
  }
}

pub struct FaceToHalfEdgeIter<'a, M: HalfEdgeMeshData> {
  mesh: &'a HalfEdgeMesh<M>,
  start: Handle<HalfEdge<M>>,
  last: Option<Handle<HalfEdge<M>>>,
}

impl<'a, M: HalfEdgeMeshData> Iterator for FaceToHalfEdgeIter<'a, M> {
  type Item = EdgeIterItem<'a, M>;

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(last) = self.last {
      let edge = &self.mesh[last];
      if edge.next != self.start {
        self.last = Some(edge.next);
        Some((edge, edge.next))
      } else {
        None
      }
    } else {
      self.last = Some(self.start);
      let edge = &self.mesh[self.start];
      Some((edge, self.start))
    }
  }
}
