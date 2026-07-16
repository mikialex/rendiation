use crate::*;

pub struct SceneBoundingComputer {
  visible_no_view_dep_no_infinity_bvh: SceneBVHResultView,
  visible_no_view_dep_bvh: SceneBVHResultView,
  pub sm_to_local_bbox: BoxedDynQuery<RawEntityHandle, Box3<f32>>,
  pub view_maps: BoxedDynQuery<ViewSceneModelKey, Mat4<f64>>,
  event_trace_sender: APITraceEventSender,
  //   scene_skip_sm_bounding: BoxedDynMultiQuery<RawEntityHandle, RawEntityHandle>,
  //   world_mats: BoxedDynQuery<RawEntityHandle, Mat4<f64>>,
  scene_model_visible: BoxedDynQuery<RawEntityHandle, bool>,
  scene_model_infinity: BoxedDynQuery<RawEntityHandle, bool>,
}

impl Drop for SceneBoundingComputer {
  fn drop(&mut self) {
    self
      .event_trace_sender
      .emit(&RendiationCxAPITraceEvent::DropBoundingComputer);
  }
}

impl SceneBoundingComputer {
  // note, currently if we have view dep object that marked as infinity,
  // use consider_view_dep false, consider_infinity true, will not consider this object
  // this behavior might be changed in future
  pub fn get_or_compute_scene_bounding(
    &self,
    scene: EntityHandle<SceneEntity>,
    consider_view_dep: Option<u64>,
    consider_infinity: bool,
  ) -> Box3<f32> {
    let scene_raw = scene.into_raw();
    self
      .event_trace_sender
      .emit(&RendiationCxAPITraceEvent::SceneBoundingQuery {
        scene: scene_raw,
        active_view_id: consider_view_dep,
      });

    let bvh = if consider_infinity {
      &self.visible_no_view_dep_bvh
    } else {
      &self.visible_no_view_dep_no_infinity_bvh
    };

    let mut r = bvh.bvh.get_root_aabb(scene_raw).unwrap_or(Box3::empty());

    if let Some(active_view_id) = consider_view_dep {
      // we assume this kind of case is not too common
      for ((view_id, sm), mat) in self.view_maps.iter_key_value() {
        if view_id == active_view_id {
          if self.scene_model_visible.access(&sm) != Some(true) {
            continue;
          }
          let is_infinity = self.scene_model_infinity.access(&sm) == Some(true);
          if !consider_infinity && is_infinity {
            continue;
          }

          if let Some(other) = self.sm_to_local_bbox.access(&sm) {
            let world_aabb = other.apply_matrix_into(mat.into_f32());
            r.expand_by_other(world_aabb);
          }
        }
      }
    }

    //   if let Some(iter) = self
    //     .scene_skip_sm_bounding
    //     .access_multi(scene.raw_handle_ref())
    //   {
    //     for sm in iter {
    //       if let Some(local) = self.sm_to_local_bbox.access(&sm) {
    //         if let Some(mat) = self.world_mats.access(&sm) {
    //           let world_aabb = local.apply_matrix_into(mat.into_f32());
    //           r.expand_by_other(world_aabb);
    //         }
    //       }
    //     }
    //   }

    r
  }
}

pub fn use_bounding_computer(cx: &mut ViewerAPICx) -> Option<SceneBoundingComputer> {
  expect_tracing_event_emitter().emit(&RendiationCxAPITraceEvent::CreateBoundingComputer);

  let f = cx.viewer.font_system.clone();

  let sm_local_bounding = cx
    .use_shared_dual_query_view(SceneModelLocalBounding(f.clone()))
    .use_assure_result(cx);

  let view_maps = cx
    .use_shared_dual_query_view(SceneModelViewDependentTransformOccShare(
      cx.viewer.ndc().clone(),
      cx.viewer.viewport_map.clone(),
    ))
    .use_assure_result(cx);

  //   let skip_sm_bounding = cx
  //     .use_dual_query::<SceneModelIsInfinity>()
  //     .dual_query_filter_map(|v| v.then_some(()));

  //   let has_view_dep = cx
  //     .use_dual_query::<SceneModelViewDependentTransformOcc>()
  //     .dual_query_filter_map(|v| v.is_some().then_some(()));

  //   let scene_skip_sm_bounding = cx
  //     .use_dual_query::<SceneModelBelongsToScene>()
  //     .dual_query_filter_map(|v| v)
  //     .dual_query_boxed()
  //     .dual_query_filter_by_set(skip_sm_bounding)
  //     .dual_query_boxed()
  //     .dual_query_union(has_view_dep, |(s, has_view_dep)| match (s, has_view_dep) {
  //       (Some(v), None) => Some(v),
  //       _ => None,
  //     })
  //     .dual_query_boxed()
  //     .use_dual_query_hash_many_to_one(cx)
  //     .use_assure_result(cx);

  //   let world_mats = use_global_node_world_mat_view(cx).use_assure_result(cx);

  // note, node visible is not considered for now.
  let sm_visible = cx.use_dual_query::<SceneModelVisible>();
  let visible_has_bounding = cx
    .use_shared_dual_query(SceneModelWorldBounding(f))
    .dual_query_filter_map(|v| v)
    .dual_query_union(sm_visible, |(bbox, visible)| match (bbox, visible) {
      (Some(bbox), Some(true)) => Some(bbox),
      _ => None,
    })
    .dual_query_boxed();

  let (visible_has_bounding, visible_has_bounding_) = visible_has_bounding.fork();

  let sm_infinity = cx.use_dual_query::<SceneModelIsInfinity>();
  let visible_no_infinity = visible_has_bounding
    .dual_query_union(sm_infinity, |(bbox, infinity)| match (bbox, infinity) {
      (Some(bbox), Some(false)) => Some(bbox),
      _ => None,
    })
    .dual_query_boxed();

  let visible_no_view_dep_no_infinity_bvh = use_bvh(cx, visible_no_infinity);
  let visible_no_view_dep_bvh = use_bvh(cx, visible_has_bounding_);

  cx.when_resolve_stage(|| SceneBoundingComputer {
    sm_to_local_bbox: sm_local_bounding.expect_resolve_stage(),
    view_maps: view_maps.expect_resolve_stage(),
    event_trace_sender: expect_tracing_event_emitter(),
    visible_no_view_dep_no_infinity_bvh: visible_no_view_dep_no_infinity_bvh
      .into_resolve_stage()
      .unwrap(),
    visible_no_view_dep_bvh: visible_no_view_dep_bvh.into_resolve_stage().unwrap(),
    scene_model_visible: get_db_view::<SceneModelVisible>().into_boxed(),
    scene_model_infinity: get_db_view::<SceneModelIsInfinity>().into_boxed(),
    // world_mats: world_mats.expect_resolve_stage(),
    // scene_skip_sm_bounding: scene_skip_sm_bounding
    //   .expect_resolve_stage()
    //   .inv_view_view_delta()
    //   .0
    //   .into_boxed_multi(),
  })
}

fn use_bvh(
  cx: &mut impl DBHookCxLike,
  bounding: UseResult<BoxedDynDualQuery<RawEntityHandle, Box3<f64>>>,
) -> UseResult<SceneBVHResultView> {
  let (bounding, b) = bounding.fork();
  let margin = b.dual_query_map(|_| 0.);
  let bounding = bounding.dual_query_map(|v| Some(v));

  rendiation_dynamic_bvh_scene::use_scene_dynamic_bvh(cx, bounding, margin)
}
