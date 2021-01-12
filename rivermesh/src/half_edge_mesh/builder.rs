use arena::{Arena, Handle};

use crate::{HalfEdgeMesh, HalfEdgeVertex};

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
}

pub enum HalfEdgeOperationError {
  NonManifoldOperation,
  InvalidFaceConstructionInput,
}
use HalfEdgeOperationError::*;

pub struct HalfEdgeMeshBuilder<V, HE, F> {
  mesh: HalfEdgeMesh<V, HE, F>,
  pub building_vertices: Arena<BuildingVertex<V, HE, F>>, // this actually not allow remove, so we should not use arena!
}

impl<V, HE, F> HalfEdgeMeshBuilder<V, HE, F> {
  pub fn new() -> Self {
    Self {
      mesh: HalfEdgeMesh::new(),
      building_vertices: Arena::new(),
    }
  }

  pub fn push_any_face(
    &mut self,
    path: &[BuildingVertex<V, HE, F>],
  ) -> Result<(), HalfEdgeOperationError> {
    if path.len() < 3 {
      return Err(InvalidFaceConstructionInput);
    }
    Ok(())
  }

  pub fn push_triangle_face(
    &mut self,
    a: BuildingVertex<V, HE, F>,
    b: BuildingVertex<V, HE, F>,
    c: BuildingVertex<V, HE, F>,
  ) -> Result<(), HalfEdgeOperationError> {
    Ok(())
  }

  pub fn done(self) -> HalfEdgeMesh<V, HE, F> {
    self.mesh
  }
}
