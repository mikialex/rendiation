use rendiation_geometry::Box3;

use crate::*;

pub trait MeshLocalBoundingCompute {
  fn build_local_bound_collection() -> impl ReactiveKVCollection<u32, Option<Box3>>;
}

pub trait SceneMeshLocalBoundingCompute {
  fn build_mesh_model_relation() -> impl ReactiveOneToManyRefBookKeeping<u32, u32>;
}
