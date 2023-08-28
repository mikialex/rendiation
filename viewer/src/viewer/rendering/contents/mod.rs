use crate::*;

mod fatline;
pub use fatline::*;
mod model_overrides;
pub use model_overrides::*;
use rendiation_mesh_core::{
  vertex::Vertex, DynIndexContainer, GroupedMesh, IndexedMesh, IntersectAbleGroupedMesh,
  TriangleList,
};

pub fn register_viewer_extra_scene_features() {
  register_material::<SharedIncrementalSignal<FatLineMaterial>>();

  register_mesh::<SharedIncrementalSignal<FatlineMesh>>();
  register_mesh::<
    SharedIncrementalSignal<GroupedMesh<IndexedMesh<TriangleList, Vec<Vertex>, DynIndexContainer>>>,
  >();
}

fn register_mesh<T>()
where
  T: AsRef<dyn GlobalIdentified>
    + AsMut<dyn GlobalIdentified>
    + AsRef<dyn WebGPUSceneMesh>
    + AsMut<dyn WebGPUSceneMesh>
    + AsRef<dyn IntersectAbleGroupedMesh>
    + AsMut<dyn IntersectAbleGroupedMesh>
    // + AsRef<dyn WatchableSceneMeshLocalBounding>
    // + AsMut<dyn WatchableSceneMeshLocalBounding>
    + 'static,
{
  register_core_mesh_features::<T>();
  register_webgpu_mesh_features::<T>();
}

fn register_material<T>()
where
  T: AsRef<dyn GlobalIdentified>
    + AsMut<dyn GlobalIdentified>
    + AsRef<dyn WebGPUSceneMaterial>
    + AsMut<dyn WebGPUSceneMaterial>
    + 'static,
{
  register_core_material_features::<T>();
  register_webgpu_material_features::<T>();
}
