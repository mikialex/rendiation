use crate::*;

pub trait ScenePicker {
  fn ray_query_nearest(
    &self,
    idx: EntityHandle<SceneEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint<f64>>;
}
