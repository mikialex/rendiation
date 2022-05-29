use rendiation_geometry::Triangle;

use crate::*;

use HalfEdgeBuildError::*;
use NoneManifoldError::*;

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

  // mesh
  //   .vertices
  //   .get(a)
  //   .unwrap()
  //   .iter_half_edge(&mesh)
  //   .for_each(|(he, _)| {
  //     he.debug(&mesh);
  //   });

  // let err = builder.build_triangle_face(Triangle::new(
  //   BuildingVertex::Attached(b),
  //   BuildingVertex::Attached(a),
  //   BuildingVertex::Detached("_"),
  // ));
  // assert_eq!(err, Err(NonManifoldOperation(DanglingEdge)))
}
