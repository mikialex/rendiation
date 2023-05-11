use std::fmt::Debug;

use arena::Handle;

use super::{HalfEdgeFace, HalfEdgeVertex};
use crate::{HalfEdgeMesh, HalfEdgeMeshData};

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

  /// next half-edge around the face
  pub(super) prev: Handle<HalfEdge<M>>,
}

impl<M: HalfEdgeMeshData> HalfEdge<M> {
  pub fn get_by_two_points(
    mesh: &HalfEdgeMesh<M>,
    from: Handle<HalfEdgeVertex<M>>,
    to: Handle<HalfEdgeVertex<M>>,
  ) -> Option<Handle<HalfEdge<M>>> {
    if from == to {
      return None;
    }
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
    self.prev
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

  pub(crate) fn debug(&self, mesh: &HalfEdgeMesh<M>)
  where
    M::Vertex: Debug,
    M::HalfEdge: Debug,
  {
    println!("debug half edge: {:?}", self.data);
    let start = &mesh.vertices.get(self.vert).unwrap().data;
    println!("  start: {:?}", start);
    let end = &mesh.vertices.get(self.end(mesh)).unwrap().data;
    println!("  end  : {:?}", end);
  }
}
