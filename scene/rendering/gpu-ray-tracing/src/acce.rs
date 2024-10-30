use crate::*;

type Blas = BottomLevelAccelerationStructureHandle;

pub fn mesh_group_to_blas(
) -> impl ReactiveQuery<Key = (EntityHandle<AttributesMeshEntity>, u32), Value = Blas> {
  EmptyQuery::default()
}

pub fn scene_model_to_tlas_instance(
) -> impl ReactiveQuery<Key = EntityHandle<SceneModelEntity>, Value = (Blas, Mat4<f32>)> {
  EmptyQuery::default()
}

pub fn scene_to_tlas(
) -> impl ReactiveQuery<Key = EntityHandle<SceneModelEntity>, Value = TlasInstance> {
  EmptyQuery::default()
}
