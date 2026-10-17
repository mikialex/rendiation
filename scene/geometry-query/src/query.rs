//! Query drivers. Every high level picking API is a combination of a candidate provider, a scene
//! model picker and a visitor.

use crate::*;

/// The generic scene ray query, all other ray query functions are built on this.
pub fn ray_query_scene(
  picker: &dyn SceneModelPicker,
  candidates: &dyn SceneModelCandidateProvider,
  scene: EntityHandle<SceneEntity>,
  query: &SceneRayQuery,
  v: &mut dyn RayHitVisitor,
) -> ControlFlow<()> {
  let t_max = v.t_max();
  candidates.traverse_ray(scene, query, t_max, &mut |candidate| {
    if candidate.t_enter <= v.t_max() {
      let req = SceneModelRayQuery {
        idx: candidate.model,
        override_world_mat: None,
        ctx: query,
        ignore_pre_check: false,
      };
      picker.ray_query(req, &mut *v)?;
    }
    ControlFlow::Continue(v.t_max())
  })
}

pub fn pick_nearest(
  picker: &dyn SceneModelPicker,
  candidates: &dyn SceneModelCandidateProvider,
  scene: EntityHandle<SceneEntity>,
  query: &SceneRayQuery,
) -> Option<SceneRayHit> {
  let mut v = NearestRayHit::default();
  let _ = ray_query_scene(picker, candidates, scene, query, &mut v);
  v.nearest
}

/// the nearest hit that passes the filter, for example the clipping plane filter
pub fn pick_nearest_filtered(
  picker: &dyn SceneModelPicker,
  candidates: &dyn SceneModelCandidateProvider,
  scene: EntityHandle<SceneEntity>,
  query: &SceneRayQuery,
  filter: impl FnMut(&SceneRayHit) -> bool,
) -> Option<SceneRayHit> {
  let mut v = FilteredRayHits {
    inner: NearestRayHit::default(),
    filter,
  };
  let _ = ray_query_scene(picker, candidates, scene, query, &mut v);
  v.inner.nearest
}

/// all hits sorted by t ascending
pub fn pick_all(
  picker: &dyn SceneModelPicker,
  candidates: &dyn SceneModelCandidateProvider,
  scene: EntityHandle<SceneEntity>,
  query: &SceneRayQuery,
) -> Vec<SceneRayHit> {
  let mut v = AllRayHits::default();
  let _ = ray_query_scene(picker, candidates, scene, query, &mut v);
  v.into_sorted()
}

/// model level range pick
pub fn range_pick_models(
  picker: &dyn SceneModelPicker,
  candidates: &dyn SceneModelCandidateProvider,
  scene: EntityHandle<SceneEntity>,
  frustum: &SceneFrustumQuery,
  policy: ObjectTestPolicy,
  results: &mut Vec<EntityHandle<SceneModelEntity>>,
) {
  let _ = candidates.traverse_frustum(scene, frustum, &mut |candidate| {
    let req = SceneModelFrustumQuery {
      idx: candidate.model,
      override_world_mat: None,
      frustum,
      policy,
      known_inside: candidate.known_inside,
      ignore_pre_check: false,
    };
    let mut v = ModelFrustumVisitor::new(policy);
    let _ = picker.frustum_query(req, &mut v);
    if v.selected() {
      results.push(candidate.model);
    }
    ControlFlow::Continue(())
  });
}

/// sub primitive level range pick of a single model
pub fn range_pick_sub_primitives(
  picker: &dyn SceneModelPicker,
  model: EntityHandle<SceneModelEntity>,
  frustum: &SceneFrustumQuery,
  policy: ObjectTestPolicy,
  results: &mut Vec<SubPrimitiveId>,
) {
  let req = SceneModelFrustumQuery {
    idx: model,
    override_world_mat: None,
    frustum,
    policy,
    known_inside: false,
    ignore_pre_check: false,
  };
  let _ = picker.frustum_query(req, &mut SubPrimitiveVisitor { results });
}
