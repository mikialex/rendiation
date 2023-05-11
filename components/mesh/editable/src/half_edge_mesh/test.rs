use rendiation_geometry::Triangle;
use HalfEdgeBuildError::*;
use NoneManifoldError::*;

use crate::*;

struct TestMeshSchema;

impl HalfEdgeMeshData for TestMeshSchema {
  type Face = &'static str;
  type HalfEdge = &'static str;
  type Vertex = &'static str;
}

pub struct TriangleBuildSource<M: HalfEdgeMeshData> {
  a: BuildingVertex<M>,
  b: BuildingVertex<M>,
  c: BuildingVertex<M>,
  ab: M::HalfEdge,
  bc: M::HalfEdge,
  ca: M::HalfEdge,
  face: M::Face,
}

#[test]
fn build_mesh() {
  let mut mesh = HalfEdgeMesh::<TestMeshSchema>::new();

  assert_eq!(mesh.face_count(), 0);

  let (a, b, c) = mesh
    .build_triangle_face(Triangle::new(
      BuildingVertex::Detached("a"),
      BuildingVertex::Detached("b"),
      BuildingVertex::Detached("c"),
    ))
    .unwrap()
    .into();

  assert_eq!(mesh[a].half_edge_connected_count(&mesh), 2);
  assert_eq!(mesh[b].half_edge_connected_count(&mesh), 2);
  assert_eq!(mesh[c].half_edge_connected_count(&mesh), 2);

  // todo fix bug
  // assert_eq!(mesh[a].face_connected_count(&mesh), 1);
  // assert_eq!(mesh[b].face_connected_count(&mesh), 1);
  // assert_eq!(mesh[c].face_connected_count(&mesh), 1);

  assert_eq!(mesh[a].is_boundary_vertex(&mesh), true);
  assert_eq!(mesh[b].is_boundary_vertex(&mesh), true);
  assert_eq!(mesh[c].is_boundary_vertex(&mesh), true);

  assert_eq!(mesh.face_count(), 1);

  let (b, a, d) = mesh
    .build_triangle_face(Triangle::new(
      BuildingVertex::Attached(b),
      BuildingVertex::Attached(a),
      BuildingVertex::Detached("d"),
    ))
    .unwrap()
    .into();

  assert_eq!(mesh[a].half_edge_connected_count(&mesh), 4);
  assert_eq!(mesh[b].half_edge_connected_count(&mesh), 4);
  assert_eq!(mesh[c].half_edge_connected_count(&mesh), 2);
  assert_eq!(mesh[d].half_edge_connected_count(&mesh), 2);

  assert_eq!(mesh[a].is_boundary_vertex(&mesh), true);
  assert_eq!(mesh[b].is_boundary_vertex(&mesh), true);
  assert_eq!(mesh[c].is_boundary_vertex(&mesh), true);
  assert_eq!(mesh[d].is_boundary_vertex(&mesh), true);

  assert_eq!(mesh.face_count(), 2);

  // mesh
  //   .vertices
  //   .get(a)
  //   .unwrap()
  //   .iter_half_edge(&mesh)
  //   .for_each(|(he, _)| {
  //     he.debug(&mesh);
  //   });

  let err = mesh.build_triangle_face(Triangle::new(
    BuildingVertex::Attached(b),
    BuildingVertex::Attached(a),
    BuildingVertex::Detached("_"),
  ));
  assert_eq!(err.is_err(), true);

  let err = mesh.build_triangle_face(Triangle::new(
    BuildingVertex::Attached(b),
    BuildingVertex::Detached("_"),
    BuildingVertex::Detached("_"),
  ));
  assert_eq!(err.is_err(), true);
}
