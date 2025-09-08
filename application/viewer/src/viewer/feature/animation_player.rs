use fast_hash_collection::FastHashSet;

use crate::*;

// todo, currently all animation is played at the same time, this should be configurable
// because they may cause conflict result.
pub fn use_animation_player(cx: &mut ViewerCx) {
  let channel_of_animation = cx
    .use_db_rev_ref::<SceneAnimationChannelBelongToAnimation>()
    .use_assure_result(cx);

  let (cx, mutation) = cx.use_plain_state::<Option<SceneAnimationMutation>>();

  let (cx, active_animations) =
    cx.use_plain_state::<FastHashSet<EntityHandle<SceneAnimationEntity>>>();

  match &mut cx.stage {
    ViewerCxStage::EventHandling { .. } => {
      let channel_of_animation = channel_of_animation.expect_resolve_stage();

      let m = compute_mutation(
        active_animations,
        channel_of_animation,
        cx.absolute_seconds_from_start,
      );
      *mutation = Some(m);
    }
    ViewerCxStage::SceneContentUpdate { writer, .. } => {
      if let Some(m) = mutation.take() {
        m.apply(writer);
      }
    }
    _ => {}
  }

  if let ViewerCxStage::Gui { egui_ctx, global } = &mut cx.stage {
    let opened = global.features.entry("animation").or_insert(false);

    egui::Window::new("Animation")
      .open(opened)
      .vscroll(true)
      .show(egui_ctx, |ui| {
        let animations = get_db_view_typed_foreign::<SceneAnimationBelongsToScene>();
        let animation_name = get_db_view_typed::<LabelOf<SceneAnimationEntity>>();

        if !animations.is_empty() {
          ui.label("animations in target scene:");
          for (animation, scene) in animations.iter_key_value() {
            if scene == cx.viewer.scene.scene {
              ui.label(animation_name.access(&animation).unwrap());
              let mut enable = active_animations.contains(&animation);
              ui.checkbox(&mut enable, "play");
              if enable {
                active_animations.insert(animation);
              } else {
                active_animations.remove(&animation);
              }
            }
          }
        } else {
          ui.label("no animations found in target scene");
        }
      });
  }
}

/// currently we implement per channel standalone looping behavior, i'm not sure how spec say about it
/// https://github.com/KhronosGroup/glTF/issues/1179
fn compute_mutation(
  active_animations: &mut FastHashSet<EntityHandle<SceneAnimationEntity>>,
  channel_of_animation: RevRefForeignKeyRead,
  absolute_world_time_in_sec: f32,
) -> SceneAnimationMutation {
  let channel_of_animation =
    channel_of_animation.mark_foreign_key::<SceneAnimationChannelBelongToAnimation>();

  let channel_reader = global_entity_of::<SceneAnimationChannelEntity>().entity_reader();
  let buffer_reader = global_entity_component_of::<BufferEntityData>().read();
  let input_read = SceneBufferViewReadView::<SceneAnimationChannelInput>::new_from_global();
  let output_read = SceneBufferViewReadView::<SceneAnimationChannelOutput>::new_from_global();

  let mut mutations = Vec::new();
  let target = global_entity_component_of::<SceneAnimationChannelTargetNode>().read_foreign_key();
  let mut to_remove = Vec::new();
  for animation in active_animations.iter() {
    if let Some(animation) = channel_of_animation.access_multi(animation) {
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
    } else {
      to_remove.push(*animation);
    }
  }
  for animation in to_remove {
    active_animations.remove(&animation);
  }

  SceneAnimationMutation(mutations)
}

struct SceneAnimationMutation(Vec<(InterpolationItem, EntityHandle<SceneNodeEntity>)>);

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
          position = vec3.into_f64();
        }
        InterpolationItem::Scale(vec3) => {
          scale = vec3.into_f64();
        }
        InterpolationItem::Quaternion(quat) => {
          rotation = quat.into_f64();
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
