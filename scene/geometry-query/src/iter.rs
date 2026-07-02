use crate::*;

/// The TLAS abstraction for picking
pub trait SceneModelIterProvider {
  fn create_frustum_scene_model_iter<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    frustum: &'a SceneFrustumQuery,
  ) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + 'a>;
  fn create_ray_scene_model_iter<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    ctx: &'a SceneRayQuery,
  ) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + 'a>;
}

pub fn pick_models_all(
  model_impl: &dyn SceneModelPicker,
  models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
  cx: &SceneRayQuery,
  results: &mut Vec<MeshBufferHitPoint<f64>>,
  models_results: &mut Vec<EntityHandle<SceneModelEntity>>,
  local_result_scratch: &mut Vec<MeshBufferHitPoint<f32>>,
  ignore_pre_check: bool,
) {
  for m in models {
    let len = results.len();
    if model_impl
      .ray_query_all(m, None, cx, results, local_result_scratch, ignore_pre_check)
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
  ignore_pre_check: bool,
) -> Option<(HitPoint3D<f64>, EntityHandle<SceneModelEntity>)> {
  let mut nearest: Option<(HitPoint3D<f64>, EntityHandle<SceneModelEntity>)> = None;
  for m in models {
    if let Some(hit) = model_impl.ray_query_nearest(m, None, cx, ignore_pre_check) {
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

pub fn range_pick_models(
  model_impl: &dyn SceneModelPicker,
  models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
  frustum: &SceneFrustumQuery,
  policy: ObjectTestPolicy,
  add_results: &mut dyn FnMut(EntityHandle<SceneModelEntity>),
  ignore_pre_check: bool,
) {
  for m in models {
    if let Some(true) = model_impl.frustum_query(m, None, frustum, policy, ignore_pre_check) {
      add_results(m);
    }
  }
}
