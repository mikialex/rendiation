use crate::*;

pub fn pick_models_all(
  model_impl: &dyn SceneModelPicker,
  models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
  cx: &SceneRayQuery,
  results: &mut Vec<MeshBufferHitPoint<f64>>,
  models_results: &mut Vec<EntityHandle<SceneModelEntity>>,
  local_result_scratch: &mut Vec<MeshBufferHitPoint<f32>>,
) {
  for m in models {
    let len = results.len();
    if model_impl
      .ray_query_all(m, cx, results, local_result_scratch)
      .is_some()
    {
      for _ in len..results.len() {
        models_results.push(m);
      }
    }
  }
}

pub fn pick_models_nearest(
  model_impl: &dyn SceneModelPicker,
  models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
  cx: &SceneRayQuery,
) -> Option<(HitPoint3D<f64>, EntityHandle<SceneModelEntity>)> {
  let mut nearest: Option<(HitPoint3D<f64>, EntityHandle<SceneModelEntity>)> = None;
  for m in models {
    if let Some(hit) = model_impl.ray_query_nearest(m, cx) {
      let hit = hit.hit;
      if let Some(n) = nearest {
        if hit.is_near_than(&n.0) {
          nearest = Some((hit, m));
        }
      } else {
        nearest = Some((hit, m));
      }
    }
  }
  nearest
}
