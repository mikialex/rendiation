use futures::StreamExt;
use rendiation_geometry::Box3;

use crate::*;

pub trait MeshLocalBoundingCompute {
  fn build_local_bound_collection() -> impl ReactiveCollection<u32, Option<Box3>>;
}

// pub trait SceneMeshLocalBoundingCompute {
//   fn build_mesh_model_relation() -> impl OneToManyRefBookKeeping<u32, u32>;
// }

// #[allow(clippy::single_match)]
// #[allow(clippy::collapsible_match)]
// pub fn watch_ref_change(
//   model: &IncrementalSignalStorage<StandardModel>,
// ) -> impl Stream<Item = Vec<ManyToOneReferenceChange<u32, u32>>> {
//   model
//     .single_listen_by(|change, collector| match change {
//       MaybeDeltaRef::Delta(delta) => match delta {
//         StandardModelDelta::mesh(mesh) => match mesh {
//           MeshEnum::AttributesMesh(mesh) => collector(Some(mesh.alloc_index())),
//           _ => {}
//         },
//         _ => {}
//       },
//       MaybeDeltaRef::All(all) => match &all.mesh {
//         MeshEnum::AttributesMesh(mesh) => collector(Some(mesh.alloc_index())),
//         _ => {}
//       },
//     })
//     .map(|deltas| {
//       deltas
//         .into_iter()
//         .map(CollectionDelta::into_ref_change)
//         .collect::<Vec<_>>()
//     })
// }
