use crate::*;

/// currently we implement per channel standalone looping behavior, i'm not sure how spec say about it
/// https://github.com/KhronosGroup/glTF/issues/1179
pub struct SceneAnimationsPlayer {
  animation_of_scene: BoxedDynReactiveOneToManyRelation<
    EntityHandle<SceneEntity>,
    EntityHandle<SceneAnimationEntity>,
  >,
  channel_of_animation: BoxedDynReactiveOneToManyRelation<
    EntityHandle<SceneAnimationEntity>,
    EntityHandle<SceneAnimationChannelEntity>,
  >,
}

impl SceneAnimationsPlayer {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    let animation_of_scene = global_rev_ref().watch_inv_ref::<SceneAnimationBelongsToScene>();
    let channel_of_animation =
      global_rev_ref().watch_inv_ref::<SceneAnimationChannelBelongToAnimation>();

    Self {
      animation_of_scene: Box::new(animation_of_scene),
      channel_of_animation: Box::new(channel_of_animation),
    }
  }

  pub fn compute_mutation(
    &mut self,
    cx: &mut Context,
    target_scene: EntityHandle<SceneEntity>,
    absolute_world_time_in_sec: f32,
  ) -> SceneAnimationMutation {
    let (_, _, animation_of_scene) = self.animation_of_scene.poll_changes_with_inv_dyn(cx);
    let (_, _, channel_of_animation) = self.channel_of_animation.poll_changes_with_inv_dyn(cx);

    let channel_reader = global_entity_of::<SceneAnimationChannelEntity>().entity_reader();
    let buffer_reader = global_entity_component_of::<BufferEntityData>().read();
    let input_read = SceneBufferViewReadView::<SceneAnimationChannelInput>::new_from_global();
    let output_read = SceneBufferViewReadView::<SceneAnimationChannelOutput>::new_from_global();

    let mut mutations = Vec::new();
    let target = global_entity_component_of::<SceneAnimationChannelTargetNode>().read_foreign_key();
    if let Some(animations_in_scene) = animation_of_scene.access_multi(&target_scene) {
      for animation in animations_in_scene {
        if let Some(animation) = channel_of_animation.access_multi(&animation) {
          for channel in animation {
            let new_sampler = AnimationSampler {
              interpolation: channel_reader.read::<SceneAnimationChannelInterpolation>(channel),
              field: channel_reader.read::<SceneAnimationChannelField>(channel),
              input: scene_buffer_view_into_attribute(
                input_read.read_view(channel).unwrap(),
                &buffer_reader,
              )
              .unwrap(),
              output: scene_buffer_view_into_attribute(
                output_read.read_view(channel).unwrap(),
                &buffer_reader,
              )
              .unwrap(),
            };

            let target = target.get(channel).unwrap();
            let action = new_sampler
              .sample_animation(absolute_world_time_in_sec)
              .unwrap();
            mutations.push((action, target))
          }
        }
      }
    }

    SceneAnimationMutation(mutations)
  }
}

pub struct SceneAnimationMutation(Vec<(InterpolationItem, EntityHandle<SceneNodeEntity>)>);

impl SceneAnimationMutation {
  pub fn apply(self, scene: &mut SceneWriter) {
    for (action, target) in self.0 {
      let target_node_mat = scene
        .node_writer
        .try_read::<SceneNodeLocalMatrixComponent>(target)
        .unwrap();
      let (mut position, mut rotation, mut scale) = target_node_mat.decompose();
      match action {
        InterpolationItem::Position(vec3) => {
          position = vec3;
        }
        InterpolationItem::Scale(vec3) => {
          scale = vec3;
        }
        InterpolationItem::Quaternion(quat) => {
          rotation = quat;
        }
        InterpolationItem::MorphTargetWeights(_) => {
          // not supported yet
        }
      }
      let new_mat = Mat4::compose(position, rotation, scale);
      scene.set_local_matrix(target, new_mat);
    }
  }
}
