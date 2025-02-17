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
