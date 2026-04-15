use crate::*;

pub struct SceneModelLocalBounding(pub Arc<RwLock<FontSystem>>);

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for SceneModelLocalBounding {
  share_provider_hash_type_id! {}

  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f32>>;

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let att_mesh_std_sm_bounding = cx.use_shared_dual_query(
      SceneModelByAttributesMeshStdModelLocalBounding(viewer_mesh_input),
    );

    let wide_line_sm_bounding = cx.use_shared_dual_query(WideLineSceneModelLocalBounding);
    let wide_point_sm_bounding = cx.use_shared_dual_query(WideStyledPointsSceneModelLocalBounding);
    let text3d_sm_bounding =
      cx.use_shared_dual_query(Text3dSceneModelLocalBounding(self.0.clone()));

    let extra = wide_line_sm_bounding
      .dual_query_select(wide_point_sm_bounding)
      .dual_query_boxed()
      .dual_query_select(text3d_sm_bounding)
      .dual_query_boxed();

    att_mesh_std_sm_bounding.dual_query_select(extra)
  }
}

pub struct SceneModelWorldBounding(pub Arc<RwLock<FontSystem>>);

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for SceneModelWorldBounding {
  share_provider_hash_type_id! {}

  // return None if the box is dynamic
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Option<Box3<f64>>>;

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let scene_model_world_mat = cx.use_shared_dual_query(GlobalSceneModelWorldMatrix);
    let all_model_local_bounding =
      cx.use_shared_dual_query(SceneModelLocalBounding(self.0.clone()));

    let models_contains_view_dep = cx
      .use_dual_query::<SceneModelViewDependentTransformOcc>()
      .dual_query_filter_map(|v| v);

    scene_model_world_mat
      .dual_query_boxed()
      .dual_query_intersect(all_model_local_bounding)
      .dual_query_map(|(mat, local)| local.into_f64().apply_matrix_into(mat))
      .dual_query_union(models_contains_view_dep, |(bbox, vp)| {
        // do subtraction
        match (bbox, vp) {
          (None, None) => None,
          (None, Some(_)) => None, // not possible, show we check?
          (Some(bbox), None) => Some(Some(bbox)),
          (Some(_), Some(_)) => Some(None),
        }
      })
      .use_dual_query_materialized_hashmap(cx, "SceneModelWorldBounding")
  }
}
