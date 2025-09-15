use crate::*;

declare_entity!(SceneSkinEntity);
declare_foreign_key!(SceneSkinRoot, SceneSkinEntity, SceneNodeEntity);

declare_entity!(SceneJointEntity);
declare_foreign_key!(SceneJointRefNode, SceneJointEntity, SceneNodeEntity);
declare_foreign_key!(SceneJointBelongToSkin, SceneJointEntity, SceneSkinEntity);
// the index should not overlapped
declare_component!(SceneJointSkinIndex, SceneJointEntity, u32);
declare_component!(
  SceneJointInverseBindMatrix,
  SceneJointEntity,
  Mat4<f32>,
  Mat4::identity()
);

pub fn register_scene_skin_data_model() {
  global_database()
    .declare_entity::<SceneSkinEntity>()
    .declare_foreign_key::<SceneSkinRoot>();

  global_database()
    .declare_entity::<SceneJointEntity>()
    .declare_foreign_key::<SceneJointRefNode>()
    .declare_foreign_key::<SceneJointBelongToSkin>()
    .declare_component::<SceneJointSkinIndex>()
    .declare_component::<SceneJointInverseBindMatrix>();
}

// SceneJointEntity
pub fn use_indexed_joints_offset_mats(
  cx: &mut impl DBHookCxLike,
) -> UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = (Mat4<f32>, u32)>> {
  // todo , impl chain query operator, this is actually one to one
  let node_many_to_one_joint = cx.use_db_rev_ref_tri_view::<SceneJointRefNode>();
  let joint_world_mats = use_global_node_world_mat(cx).fanout(node_many_to_one_joint, cx);
  joint_world_mats
    .dual_query_zip(cx.use_dual_query::<SceneJointInverseBindMatrix>())
    .dual_query_map(|(world_mat, bind_inv_mat)| (world_mat * bind_inv_mat.into_f64()).into_f32()) // todo fix precision
    .dual_query_zip(cx.use_dual_query::<SceneJointSkinIndex>())
}
