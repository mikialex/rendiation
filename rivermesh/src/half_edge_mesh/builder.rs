use arena::{Arena, Handle};

use crate::{HalfEdge, HalfEdgeMesh, HalfEdgeVertex};

pub enum BuildingVertex<V, HE, F> {
  Detached(V),
  Attached(Handle<HalfEdgeVertex<V, HE, F>>),
}

impl<V, HE, F> BuildingVertex<V, HE, F> {
  pub fn is_attached(&self) -> bool {
    match self {
      BuildingVertex::Detached(_) => false,
      BuildingVertex::Attached(_) => true,
    }
  }

  pub fn is_same_and_attached(&self, other: &Self) -> bool {
    if let BuildingVertex::Attached(handle_self) = self {
      if let BuildingVertex::Attached(handle) = other {
        return handle_self == handle;
      }
    }
    false
  }
}

pub enum HalfEdgeBuildError {
  NonManifoldOperation,
  FaceConstructionInputTooSmall,
  TriangleInputInvalid,
}
use HalfEdgeBuildError::*;

pub struct HalfEdgeMeshBuilder<V, HE, F> {
  mesh: HalfEdgeMesh<V, HE, F>,
  /// for operation recovery
  not_committed_vertices: Vec<Handle<HalfEdgeVertex<V, HE, F>>>,
  not_committed_half_edges: Vec<Handle<HalfEdge<V, HE, F>>>,
  pub building_vertices: Arena<BuildingVertex<V, HE, F>>, // this actually not allow remove, so we should not use arena!
}

impl<V, HE, F> HalfEdgeMeshBuilder<V, HE, F> {
  pub fn new() -> Self {
    Self {
      mesh: HalfEdgeMesh::new(),
      not_committed_vertices: Vec::new(),
      not_committed_half_edges: Vec::new(),
      building_vertices: Arena::new(),
    }
  }

  fn recovery(&mut self) {
    let mesh = &mut self.mesh;
    self.not_committed_vertices.drain(..).for_each(|h| {
      mesh.vertices.remove(h);
    });
    self.not_committed_half_edges.drain(..).for_each(|h| {
      mesh.half_edges.remove(h);
    });
  }

  pub fn push_any_face(
    &mut self,
    path: &[BuildingVertex<V, HE, F>],
  ) -> Result<(), HalfEdgeBuildError> {
    if path.len() < 3 {
      return Err(FaceConstructionInputTooSmall);
    }
    todo!()
  }

  pub fn push_triangle_face(
    &mut self,
    a: BuildingVertex<V, HE, F>,
    b: BuildingVertex<V, HE, F>,
    c: BuildingVertex<V, HE, F>,
  ) -> Result<(), HalfEdgeBuildError> {
    let result = self.push_triangle_face_impl(a, b, c);
    if result.is_err() {
      self.recovery()
    }
    result
  }

  pub fn push_triangle_face_impl(
    &mut self,
    a: BuildingVertex<V, HE, F>,
    b: BuildingVertex<V, HE, F>,
    c: BuildingVertex<V, HE, F>,
  ) -> Result<(), HalfEdgeBuildError> {
    if a.is_same_and_attached(&b) || b.is_same_and_attached(&c) || c.is_same_and_attached(&a) {
      return Err(TriangleInputInvalid);
    }

    Ok(())
  }

  pub fn done(self) -> HalfEdgeMesh<V, HE, F> {
    self.mesh
  }
}
