use arena::Handle;

use super::HalfEdge;
use crate::{HalfEdgeFace, HalfEdgeMesh, HalfEdgeMeshData};

#[derive(Clone, Copy)]
pub struct HalfEdgeVertex<M: HalfEdgeMeshData> {
  pub data: M::Vertex,
  /// one of the half-edges emanating from the vertex
  pub(super) edge: Handle<HalfEdge<M>>,
}

/// An iterator that iterate all half edges from this vertex
pub struct VertexToHalfEdgeIter<'a, M: HalfEdgeMeshData> {
  mesh: &'a HalfEdgeMesh<M>,
  has_meet_one_side_boundary: bool,
  has_visited_start: bool,
  start: EdgeIterItem<'a, M>,
  self_vert: &'a HalfEdgeVertex<M>,
  current: EdgeIterItem<'a, M>,
}

impl<'a, M: HalfEdgeMeshData> VertexToHalfEdgeIter<'a, M> {
  fn next_half_edge(&mut self, reverse_direction: bool) -> Option<Handle<HalfEdge<M>>> {
    let current_vert = self.mesh.vertices.get(self.current.0.vert).unwrap();

    let result = if !reverse_direction {
      if current_vert as *const _ == self.self_vert as *const _ {
        self.current.0.pair()
      } else {
        Some(self.current.0.next())
      }
    } else {
      if current_vert as *const _ == self.self_vert as *const _ {
        Some(self.current.0.prev())
      } else {
        self.current.0.pair()
      }
    };

    // update current
    if let Some(next) = result {
      self.current = (self.mesh.half_edges.get(next).unwrap(), next);
    } else {
      self.current = self.start;
    }

    result
  }
}

pub type EdgeIterItem<'a, M> = (&'a HalfEdge<M>, Handle<HalfEdge<M>>);

impl<'a, M: HalfEdgeMeshData> Iterator for VertexToHalfEdgeIter<'a, M> {
  type Item = EdgeIterItem<'a, M>;

  fn next(&mut self) -> Option<Self::Item> {
    if !self.has_visited_start {
      self.has_visited_start = true;
      return Some(self.start);
    }

    if !self.has_meet_one_side_boundary {
      if let Some(next) = self.next_half_edge(false) {
        let next_v = self.mesh.half_edges.get(next).unwrap();

        // check if we meet the start and end the iteration
        if next_v as *const _ == self.start.0 as *const _ {
          return None;
        } else {
          return Some((next_v, next));
        }
      } else {
        // go check another side
        self.has_meet_one_side_boundary = true;
      }
    }

    self
      .next_half_edge(true)
      .map(|p| (self.mesh.half_edges.get(p).unwrap(), p))
  }
}

pub type FaceIterItem<'a, M> = (&'a HalfEdgeFace<M>, Handle<HalfEdgeFace<M>>);

pub struct VertexToFaceIter<'a, M: HalfEdgeMeshData> {
  inner: VertexToHalfEdgeIter<'a, M>,
  has_visited_start: bool,
}

impl<'a, M: HalfEdgeMeshData> Iterator for VertexToFaceIter<'a, M> {
  type Item = FaceIterItem<'a, M>;

  fn next(&mut self) -> Option<Self::Item> {
    if !self.has_visited_start {
      self.has_visited_start = true;
      let face = self.inner.start.0.face();
      return Some((self.inner.mesh.faces.get(face).unwrap(), face));
    }

    self
      .inner
      .next()
      .and_then(|_| self.inner.next())
      .map(|(edge, _)| (self.inner.mesh.faces.get(edge.face).unwrap(), edge.face))
  }
}

impl<M: HalfEdgeMeshData> HalfEdgeVertex<M> {
  pub fn edge(&self) -> Handle<HalfEdge<M>> {
    self.edge
  }

  pub fn is_boundary_vertex(&self, mesh: &HalfEdgeMesh<M>) -> bool {
    self
      .iter_half_edge(mesh)
      .find(|(e, _)| e.is_border())
      .is_some()
  }

  pub fn half_edge_connected_count(&self, mesh: &HalfEdgeMesh<M>) -> usize {
    self.iter_half_edge(&mesh).count()
  }

  pub fn face_connected_count(&self, mesh: &HalfEdgeMesh<M>) -> usize {
    self.iter_face(&mesh).count()
  }

  pub fn iter_half_edge<'a>(&'a self, mesh: &'a HalfEdgeMesh<M>) -> VertexToHalfEdgeIter<'a, M> {
    let start = mesh.half_edges.get(self.edge).unwrap();
    VertexToHalfEdgeIter {
      mesh,
      has_meet_one_side_boundary: false,
      has_visited_start: false,
      start: (start, self.edge),
      self_vert: self,
      current: (start, self.edge),
    }
  }

  pub fn iter_face<'a>(&'a self, mesh: &'a HalfEdgeMesh<M>) -> VertexToFaceIter<'a, M> {
    VertexToFaceIter {
      inner: self.iter_half_edge(mesh),
      has_visited_start: false,
    }
  }
}
