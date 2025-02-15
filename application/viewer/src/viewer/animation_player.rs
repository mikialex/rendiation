use fast_hash_collection::FastHashMap;

use crate::*;

/// currently we implement per channel standalone looping behavior, i'm not sure how spec say about it
/// https://github.com/KhronosGroup/glTF/issues/1179
pub struct SceneAnimationsPlayer {
  animation_spline_cache: FastHashMap<EntityHandle<SceneAnimationChannelEntity>, AnimationSpline>,
  animation_of_scene: BoxedDynReactiveOneToManyRelation<
    EntityHandle<SceneEntity>,
    EntityHandle<SceneAnimationEntity>,
  >,
  channel_of_animation: BoxedDynReactiveOneToManyRelation<
    EntityHandle<SceneAnimationEntity>,
    EntityHandle<SceneAnimationChannelEntity>,
  >,
  max_animation_time_stamp: BoxedDynReactiveQuery<EntityHandle<SceneAnimationChannelEntity>, f32>,
}

struct AnimationSpline;

impl SceneAnimationsPlayer {
  pub fn new() -> Self {
    let animation_of_scene = global_rev_ref().watch_inv_ref::<SceneAnimationBelongsToScene>();
    let channel_of_animation =
      global_rev_ref().watch_inv_ref::<SceneAnimationChannelBelongToAnimation>();

    Self {
      animation_spline_cache: Default::default(),
      animation_of_scene: Box::new(animation_of_scene),
      channel_of_animation: Box::new(channel_of_animation),
      max_animation_time_stamp: todo!(),
    }
  }

  pub fn compute_mutation(
    &mut self,
    cx: &mut Context,
    target_scene: EntityHandle<SceneEntity>,
    scene: &mut SceneWriter,
    absolute_world_time_in_sec: f32,
  ) -> SceneAnimationMutation {
    let (_, _, animation_of_scene) = self.animation_of_scene.poll_changes_with_inv_dyn(cx);

    let (animation_channel_changes, _, channel_of_animation) =
      self.channel_of_animation.poll_changes_with_inv_dyn(cx);
    // cleanup none existed channel executor
    for (ani, delta) in animation_channel_changes.iter_key_value() {
      if delta.is_removed() {
        self.animation_spline_cache.remove(&ani);
      }
    }

    let mut mutations = Vec::new();
    let target = global_entity_component_of::<SceneAnimationChannelTargetNode>().read_foreign_key();
    for animation in animation_of_scene.access_multi(&target_scene).unwrap() {
      for channel in channel_of_animation.access_multi(&animation).unwrap() {
        let new_sampler: AnimationSampler = todo!();
        let target = target.get(channel).unwrap();
        let action = new_sampler
          .sample_animation(absolute_world_time_in_sec)
          .unwrap();
        mutations.push((action, target))
      }
    }

    SceneAnimationMutation(mutations)
  }
}

pub struct SceneAnimationMutation(Vec<(InterpolationItem, EntityHandle<SceneNodeEntity>)>);

impl SceneAnimationMutation {
  pub fn apply(self, scene: &mut SceneWriter) {
    for (action, target) in self.0 {
      match action {
        InterpolationItem::Position(vec3) => todo!(),
        InterpolationItem::Scale(vec3) => todo!(),
        InterpolationItem::Quaternion(quat) => todo!(),
        InterpolationItem::MorphTargetWeights(_) => {
          // not supported yet
        }
      }
    }
  }
}
