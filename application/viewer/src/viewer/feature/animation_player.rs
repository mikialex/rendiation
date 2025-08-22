use crate::*;

pub fn use_animation_player(cx: &mut ViewerCx) {
  let animation_of_scene = cx
    .use_db_rev_ref::<SceneAnimationBelongsToScene>()
    .use_assure_result(cx);
  let channel_of_animation = cx
    .use_db_rev_ref::<SceneAnimationChannelBelongToAnimation>()
    .use_assure_result(cx);

  let (cx, mutation) = cx.use_plain_state::<Option<SceneAnimationMutation>>();

  match &mut cx.stage {
    ViewerCxStage::EventHandling { .. } => {
      let animation_of_scene = animation_of_scene.expect_resolve_stage();
      let channel_of_animation = channel_of_animation.expect_resolve_stage();

      let m = compute_mutation(
        animation_of_scene,
        channel_of_animation,
        cx.viewer.scene.scene,
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
}

/// currently we implement per channel standalone looping behavior, i'm not sure how spec say about it
/// https://github.com/KhronosGroup/glTF/issues/1179
fn compute_mutation(
  animation_of_scene: RevRefForeignKeyRead,
  channel_of_animation: RevRefForeignKeyRead,
  target_scene: EntityHandle<SceneEntity>,
  absolute_world_time_in_sec: f32,
) -> SceneAnimationMutation {
  let animation_of_scene = animation_of_scene.mark_foreign_key::<SceneAnimationBelongsToScene>();
  let channel_of_animation =
    channel_of_animation.mark_foreign_key::<SceneAnimationChannelBelongToAnimation>();

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
