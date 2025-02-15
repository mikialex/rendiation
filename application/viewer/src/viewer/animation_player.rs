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
}

struct AnimationSpline;

impl SceneAnimationsPlayer {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    let animation_of_scene = global_rev_ref().watch_inv_ref::<SceneAnimationBelongsToScene>();
    let channel_of_animation =
      global_rev_ref().watch_inv_ref::<SceneAnimationChannelBelongToAnimation>();

    Self {
      animation_spline_cache: Default::default(),
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

    let (animation_channel_changes, _, channel_of_animation) =
      self.channel_of_animation.poll_changes_with_inv_dyn(cx);
    // cleanup none existed channel executor
    for (ani, delta) in animation_channel_changes.iter_key_value() {
      if delta.is_removed() {
        self.animation_spline_cache.remove(&ani);
      }
    }

    let channel_reader = global_entity_of::<SceneAnimationChannelEntity>().entity_reader();

    let mut mutations = Vec::new();
    let target = global_entity_component_of::<SceneAnimationChannelTargetNode>().read_foreign_key();
    for animation in animation_of_scene.access_multi(&target_scene).unwrap() {
      for channel in channel_of_animation.access_multi(&animation).unwrap() {
        let new_sampler = AnimationSampler {
          interpolation: channel_reader.read::<SceneAnimationChannelInterpolation>(channel),
          field: channel_reader.read::<SceneAnimationChannelField>(channel),
          input: read_attribute_accessor::<SceneAnimationChannelInput>(&channel_reader),
          output: read_attribute_accessor::<SceneAnimationChannelOutput>(&channel_reader),
        };

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
