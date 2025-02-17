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

pub fn indexed_joints_offset_mats(
) -> impl ReactiveQuery<Key = EntityHandle<SceneJointEntity>, Value = (Mat4<f32>, u32)> {
  // todo , impl chain query operator, this is actually one to one
  let node_many_to_one_joint = global_rev_ref().watch_inv_ref::<SceneJointRefNode>();
  let joint_world_mats = scene_node_derive_world_mat().one_to_many_fanout(node_many_to_one_joint);
  joint_world_mats
    .collective_zip(global_watch().watch::<SceneJointInverseBindMatrix>())
    .collective_map(|(world_mat, bind_inv_mat)| world_mat * bind_inv_mat)
    .collective_zip(global_watch().watch::<SceneJointSkinIndex>())
}
