use crate::*;

declare_entity!(SceneAnimationChannelEntity);
declare_foreign_key!(
  SceneAnimationChannelTargetNode,
  SceneAnimationChannelEntity,
  SceneNodeEntity
);
