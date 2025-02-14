use fast_hash_collection::FastHashMap;

use crate::*;

pub struct SceneAnimationsPlayer {
  animations: FastHashMap<EntityHandle<SceneAnimationEntity>, AnimationPlayer>,
  animation_source: BoxedDynReactiveOneToManyRelation<
    EntityHandle<SceneAnimationEntity>,
    EntityHandle<SceneEntity>,
  >,
}

impl SceneAnimationsPlayer {
  pub fn animate_targets(&mut self, scene: &mut SceneWriter, delta_time: f32) {
    for animation in self.animations.values_mut() {
      animation.animate_targets(scene, delta_time);
    }
  }

  pub fn update(&mut self, cx: &mut Context) {
    let (_, _, access) = self.animation_source.poll_changes_with_inv_dyn(cx);
    // access.access_multi(key)
  }

  pub fn egui(&mut self, ui: &mut egui::Ui) {
    todo!()
  }
}

struct AnimationPlayer {
  pub enabled: bool,
  pub current_time_stamp: f32,
  pub max_time_stamp: f32,
  pub executor: FastHashMap<EntityHandle<SceneAnimationChannelEntity>, AnimationSamplerExecutor>,
  pub target: EntityHandle<SceneNodeEntity>,
}

impl AnimationPlayer {
  pub fn set_normalized_time_stamp(&mut self, normalized_time_stamp: f32) {
    self.current_time_stamp = normalized_time_stamp * self.max_time_stamp
  }

  pub fn animate_targets(&mut self, scene: &mut SceneWriter, delta_time: f32) {
    if !self.enabled {
      return;
    }
    self.current_time_stamp += delta_time;
    self.current_time_stamp =
      (self.current_time_stamp / self.max_time_stamp).floor() * self.max_time_stamp;

    // todo update self.animation

    for sampler in self.executor.values_mut() {
      let interpolation_info = sampler.sample_animation(self.current_time_stamp).unwrap();
      match interpolation_info {
        InterpolationItem::Position(vec3) => todo!(),
        InterpolationItem::Scale(vec3) => todo!(),
        InterpolationItem::Quaternion(quat) => todo!(),
        InterpolationItem::MorphTargetWeights(_) => todo!(),
      }
    }
  }
}
