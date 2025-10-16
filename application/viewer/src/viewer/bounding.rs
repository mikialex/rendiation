use crate::*;

pub struct SceneModelWorldBounding;

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for SceneModelWorldBounding {
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f64>>;

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let att_mesh_std_sm_bounding =
      cx.use_shared_dual_query(SceneModelByAttributesMeshStdModelWorldBounding);

    let wide_line_sm_bounding = cx.use_shared_dual_query(WideLineSceneModelWorldBounding);

    att_mesh_std_sm_bounding.dual_query_select(wide_line_sm_bounding)
  }
}
