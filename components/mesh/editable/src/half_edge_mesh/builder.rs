use arena::{Arena, Handle};
use rendiation_geometry::Triangle;

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

#[derive(Debug)]
pub enum HalfEdgeBuildError {
  NonManifoldOperation(NoneManifoldError),
  FaceConstructionInputTooSmall,
  TriangleInputDegenerated,
}

#[derive(Debug)]
pub enum NoneManifoldError {
  AdjacentFaceSideInvert,
  BowtieVertex,
  DanglingPointOrEdge,
}
use HalfEdgeBuildError::*;
use NoneManifoldError::*;

pub struct HalfEdgeMeshBuilder<'a, M: HalfEdgeMeshData> {
  mesh: &'a mut HalfEdgeMesh<M>,
  /// for operation recovery
  not_committed_vertices: Vec<Handle<HalfEdgeVertex<M>>>,
  not_committed_half_edges: Vec<Handle<HalfEdge<M>>>,
}

impl<M: HalfEdgeMeshData> HalfEdgeMesh<M> {
  // pub fn build_triangle_face()
}

struct TestMeshSchema;

impl HalfEdgeMeshData for TestMeshSchema {
  type Face = ();
  type HalfEdge = ();
  type Vertex = ();
}

#[test]
fn build_mesh() {
  let mut mesh = HalfEdgeMesh::<TestMeshSchema>::new();
  let mut builder = HalfEdgeMeshBuilder::new(&mut mesh);

  let (a, b, c) = builder
    .build_triangle_face(Triangle::new(
      BuildingVertex::Detached(()),
      BuildingVertex::Detached(()),
      BuildingVertex::Detached(()),
    ))
    .unwrap()
    .into();

  let (b, a, d) = builder
    .build_triangle_face(Triangle::new(
      BuildingVertex::Attached(b),
      BuildingVertex::Attached(a),
      BuildingVertex::Detached(()),
    ))
    .unwrap()
    .into();
}

impl<'a, M: HalfEdgeMeshData> HalfEdgeMeshBuilder<'a, M> {
  pub fn new(mesh: &'a mut HalfEdgeMesh<M>) -> Self {
    Self {
      mesh,
      not_committed_vertices: Vec::new(),
      not_committed_half_edges: Vec::new(),
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

  pub fn build_any_face(&mut self, path: &[BuildingVertex<M>]) -> Result<(), HalfEdgeBuildError> {
    if path.len() < 3 {
      return Err(FaceConstructionInputTooSmall);
    }
    todo!()
  }

  pub fn build_triangle_face(
    &mut self,
    triangle: Triangle<BuildingVertex<M>>,
  ) -> Result<Triangle<Handle<HalfEdgeVertex<M>>>, HalfEdgeBuildError> {
    let result = self.build_triangle_face_impl(triangle);
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

  fn build_triangle_face_impl(
    &mut self,
    triangle: Triangle<BuildingVertex<M>>,
  ) -> Result<Triangle<Handle<HalfEdgeVertex<M>>>, HalfEdgeBuildError> {
    let a = triangle.a;
    let b = triangle.b;
    let c = triangle.c;

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

    // todo checked ok, create face
    let face = self.mesh.faces.insert(HalfEdgeFace {
      data: M::Face::default(),
      edge: ab,
    });
    self.link_half_edge(ca, ab, bc, face);
    self.link_half_edge(ab, bc, ca, face);
    self.link_half_edge(bc, ca, ab, face);

    Ok((a.0, b.0, c.0).into())
  }
}
