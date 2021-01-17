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
    let from_v = mesh.vertices.get(from).unwrap();
    from_v
      .iter_half_edge(mesh)
      .find(|(edge, _)| edge.end(mesh) == to)
      .map(|(_, e)| e)
  }

  pub fn vert(&self) -> Handle<HalfEdgeVertex<M>> {
    self.vert
  }

  pub fn next(&self) -> Handle<Self> {
    self.next
  }

  pub fn start(&self) -> Handle<HalfEdgeVertex<M>> {
    self.vert
  }
  pub fn end(&self, mesh: &HalfEdgeMesh<M>) -> Handle<HalfEdgeVertex<M>> {
    mesh.half_edges.get(self.next()).unwrap().vert()
  }

  pub fn prev(&self) -> Handle<Self> {
    // self.next().next()
    todo!()
  }

  pub fn face(&self) -> Handle<HalfEdgeFace<M>> {
    self.face
  }

  pub fn pair(&self) -> Option<Handle<Self>> {
    self.pair
  }

  pub fn is_border(&self) -> bool {
    self.pair.is_none()
  }
}
