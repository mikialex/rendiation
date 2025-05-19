use crate::*;

pub struct PickSceneBlocked;

pub fn use_pick_scene(cx: &mut ViewerCx) {
  let enable_hit_debug_log = false;
  let prefer_gpu_pick = true;

  let (cx, gpu_pick_future) =
    cx.use_plain_state::<Option<Box<dyn Future<Output = Option<u32>> + Unpin>>>();

  if let ViewerCxStage::EventHandling {
    picker,
    input,
    derived,
    ..
  } = &mut cx.stage
  {
    if let Some(f) = gpu_pick_future {
      noop_ctx!(ctx);
      if let Poll::Ready(r) = f.poll_unpin(ctx) {
        if enable_hit_debug_log {
          println!("gpu pick resolved {:?}", r);
        }

        if let Some(hit_entity_idx) = r {
          // skip the background
          if hit_entity_idx != u32::MAX {
            let hit = global_entity_of::<SceneModelEntity>()
              .entity_reader()
              .reconstruct_handle_by_idx(hit_entity_idx as usize);

            cx.viewer.scene.selected_target = hit;
          }
        }

        *gpu_pick_future = None;
      }
    }

    if cx.dyn_cx.message.take::<PickSceneBlocked>().is_some() {
      return;
    }

    if !input.state_delta.is_left_mouse_pressing() {
      return;
    }

    let scene = cx.viewer.scene.scene;

    let mut hit = None;
    let mut fallback_to_cpu = false;
    if prefer_gpu_pick && gpu_pick_future.is_none() {
      if let Some(render_size) = cx.viewer.rendering.picker.last_id_buffer_size() {
        let point = picker.normalized_position() * Vec2::from(render_size.into_f32());
        let point = point.map(|v| v.floor() as usize);
        if let Some(f) = cx.viewer.rendering.picker.pick_point_at((point.x, point.y)) {
          *gpu_pick_future = Some(f);
        }
      } else {
        fallback_to_cpu = true;
      }
    } else {
      fallback_to_cpu = true;
    }

    if fallback_to_cpu {
      let sms = &derived.sm_to_s;
      let mut main_scene_models = sms.access_multi(&scene).unwrap();
      let _hit =
        picker.pick_models_nearest(&mut main_scene_models, picker.current_mouse_ray_in_world());
      drop(main_scene_models);

      hit = if let Some(hit) = _hit {
        if enable_hit_debug_log {
          dbg!(hit);
        }
        hit.1.into()
      } else {
        None
      }
    }

    cx.viewer.scene.selected_target = hit;
  }
}
