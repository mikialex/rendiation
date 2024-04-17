use crate::*;

declare_entity!(SceneAnimationChannelEntity);
declare_foreign_key!(
  SceneAnimationChannelTargetNode,
  SceneAnimationChannelEntity,
  SceneNodeEntity
);

declare_component!(
  SceneAnimationChannelInterpolation,
  SceneAnimationChannelEntity,
  InterpolationStyle
);

declare_component!(
  SceneAnimationChannelField,
  SceneAnimationChannelEntity,
  SceneAnimationField
);

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum InterpolationStyle {
  #[default]
  Linear,
  Step,
  Cubic,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum SceneAnimationField {
  #[default]
  Position,
  Scale,
  Rotation,
  MorphTargetWeights,
}

declare_entity_associated!(SceneAnimationChannelInput, SceneAnimationChannelEntity);
declare_entity_associated!(SceneAnimationChannelOutput, SceneAnimationChannelEntity);

impl SceneBufferView for SceneAnimationChannelInput {}
impl SceneBufferView for SceneAnimationChannelOutput {}

pub fn register_scene_animation_data_model() {
  let ecg = global_database()
    .declare_entity::<SceneAnimationChannelEntity>()
    .declare_component::<SceneAnimationChannelInterpolation>()
    .declare_component::<SceneAnimationChannelField>();

  let ecg = register_scene_buffer_view::<SceneAnimationChannelInput>(ecg);
  let _ = register_scene_buffer_view::<SceneAnimationChannelOutput>(ecg);
}
