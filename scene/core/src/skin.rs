use crate::*;

declare_entity!(SceneSkinEntity);
declare_foreign_key!(SceneSkinRoot, SceneSkinEntity, SceneNodeEntity);

declare_entity!(SceneJointEntity);
declare_foreign_key!(SceneJointRefNode, SceneJointEntity, SceneNodeEntity);
declare_foreign_key!(SceneJointBelongToSkin, SceneJointEntity, SceneSkinEntity);

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
    .declare_component::<SceneJointInverseBindMatrix>();
}
