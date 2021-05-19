use arena::{Arena, Handle};

use crate::{HalfEdge, HalfEdgeFace, HalfEdgeMesh, HalfEdgeMeshData, HalfEdgeVertex};

pub enum BuildingVertex<M: HalfEdgeMeshData> {
  Detached(M::Vertex),
  Attached(Handle<HalfEdgeVertex<M>>),
}

impl<M: HalfEdgeMeshData> BuildingVertex<M> {
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
  NonManifoldOperation(NoneManifoldError),
  FaceConstructionInputTooSmall,
  TriangleInputDegenerated,
}
pub enum NoneManifoldError {
  AdjacentFaceSideInvert,
  BowtieVertex,
  DanglingPointOrEdge,
}
use HalfEdgeBuildError::*;
use NoneManifoldError::*;

pub struct HalfEdgeMeshBuilder<M: HalfEdgeMeshData> {
  mesh: HalfEdgeMesh<M>,
  /// for operation recovery
  not_committed_vertices: Vec<Handle<HalfEdgeVertex<M>>>,
  not_committed_half_edges: Vec<Handle<HalfEdge<M>>>,
  pub building_vertices: Arena<BuildingVertex<M>>, // this actually not allow remove, so we should not use arena!
}

impl<M: HalfEdgeMeshData> HalfEdgeMeshBuilder<M> {
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

  pub fn push_any_face(&mut self, path: &[BuildingVertex<M>]) -> Result<(), HalfEdgeBuildError> {
    if path.len() < 3 {
      return Err(FaceConstructionInputTooSmall);
    }
    todo!()
  }

  pub fn push_triangle_face(
    &mut self,
    a: BuildingVertex<M>,
    b: BuildingVertex<M>,
    c: BuildingVertex<M>,
  ) -> Result<(), HalfEdgeBuildError> {
    let result = self.push_triangle_face_impl(a, b, c);
    if result.is_err() {
      self.recovery()
    }
    result
  }

  fn insert_building_vertex(
    &mut self,
    v: BuildingVertex<M>,
  ) -> Result<(Handle<HalfEdgeVertex<M>>, bool), HalfEdgeBuildError> {
    match v {
      BuildingVertex::Detached(v) => {
        let vertex = HalfEdgeVertex {
          data: v,
          edge: Handle::from_raw_parts(0, 0),
        };
        let inserted = self.mesh.vertices.insert(vertex);
        self.not_committed_vertices.push(inserted);
        Ok((inserted, true))
      }
      BuildingVertex::Attached(v) => {
        let vertex = self.mesh.vertices.get(v).unwrap();
        if vertex.is_boundary_vertex(&self.mesh) {
          Err(NonManifoldOperation(DanglingPointOrEdge))
        } else {
          Ok((v, false))
        }
      }
    }
  }

  fn insert_building_half_edge(
    &mut self,
    from: (Handle<HalfEdgeVertex<M>>, bool),
    to: (Handle<HalfEdgeVertex<M>>, bool),
  ) -> Result<Handle<HalfEdge<M>>, HalfEdgeBuildError> {
    if !from.1 && !to.1 && HalfEdge::get_by_two_points(&self.mesh, from.0, to.0).is_some() {
      return Err(NonManifoldOperation(AdjacentFaceSideInvert));
    }

    let edge = HalfEdge {
      data: M::HalfEdge::default(),
      vert: from.0,
      pair: None,
      face: Handle::from_raw_parts(0, 0),
      next: Handle::from_raw_parts(0, 0),
      prev: Handle::from_raw_parts(0, 0),
    };
    let inserted = self.mesh.half_edges.insert(edge);
    self.not_committed_half_edges.push(inserted);
    Ok(inserted)
  }

  fn check_segment(
    &self,
    a: (Handle<HalfEdgeVertex<M>>, bool),
    b: (Handle<HalfEdgeVertex<M>>, bool),
    c: (Handle<HalfEdgeVertex<M>>, bool),
  ) -> Result<(), HalfEdgeBuildError> {
    if b.1 && !a.1 && !c.1 {
      return Err(NonManifoldOperation(BowtieVertex));
    }
    Ok(())
  }

  fn link_half_edge(
    &mut self,
    prev: Handle<HalfEdge<M>>,
    edge: Handle<HalfEdge<M>>,
    next: Handle<HalfEdge<M>>,
    face: Handle<HalfEdgeFace<M>>,
  ) {
    let next_vert = self.mesh.half_edges.get(edge).unwrap().vert;
    let self_vert = self.mesh.half_edges.get(edge).unwrap().vert;
    let pair = HalfEdge::get_by_two_points(&self.mesh, next_vert, self_vert);

    let edge = self.mesh.half_edges.get_mut(edge).unwrap();
    edge.next = next;
    edge.prev = prev;
    edge.face = face;
    edge.pair = pair;
  }

  pub fn push_triangle_face_impl(
    &mut self,
    a: BuildingVertex<M>,
    b: BuildingVertex<M>,
    c: BuildingVertex<M>,
  ) -> Result<(), HalfEdgeBuildError> {
    if a.is_same_and_attached(&b) || b.is_same_and_attached(&c) || c.is_same_and_attached(&a) {
      return Err(TriangleInputDegenerated);
    }

    let a = self.insert_building_vertex(a)?;
    let b = self.insert_building_vertex(b)?;
    let c = self.insert_building_vertex(c)?;
    self.check_segment(a, b, c)?;
    self.check_segment(b, c, a)?;
    self.check_segment(c, a, b)?;
    let ab = self.insert_building_half_edge(a, b)?;
    let bc = self.insert_building_half_edge(b, c)?;
    let ca = self.insert_building_half_edge(c, a)?;

    // topo checked ok, create face
    let face = self.mesh.faces.insert(HalfEdgeFace {
      data: M::Face::default(),
      edge: ab,
    });
    self.link_half_edge(ca, ab, bc, face);
    self.link_half_edge(ab, bc, ca, face);
    self.link_half_edge(bc, ca, ab, face);

    Ok(())
  }

  pub fn done(self) -> HalfEdgeMesh<M> {
    self.mesh
  }
}
