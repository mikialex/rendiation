use crate::*;

pub fn use_pick_scene(cx: &mut ViewerCx) {
  let is_webgl = cx.viewer.rendering.gpu().info().is_webgl();
  let use_gpu_pick = !is_webgl && cx.viewer.features_config.pick_scene.prefer_gpu_picking;

  let (cx, gpu_pick_future) =
    cx.use_plain_state::<Option<Box<dyn Future<Output = Option<u32>> + Unpin>>>();

  let picker = use_viewer_picker(cx);
  let sms = cx
    .use_db_rev_ref::<SceneModelBelongsToScene>()
    .use_assure_result(cx);

  if let ViewerCxStage::Gui { egui_ctx, global } = &mut cx.stage {
    let opened = global.features.entry("scene picking").or_insert(false);

    egui::Window::new("Scene picking")
      .open(opened)
      .default_size((100., 100.))
      .vscroll(true)
      .show(egui_ctx, |ui| {
        ui.checkbox(
          &mut cx.viewer.features_config.pick_scene.prefer_gpu_picking,
          "prefer gpu pick",
        );
        ui.checkbox(
          &mut cx.viewer.features_config.pick_scene.enable_hit_debug_log,
          "enable pick log",
        );
      });
  }

  let enable_hit_debug_log = cx.viewer.features_config.pick_scene.enable_hit_debug_log;

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    if let Some(f) = gpu_pick_future {
      noop_ctx!(ctx);
      if let Poll::Ready(r) = f.poll_unpin(ctx) {
        if enable_hit_debug_log {
          log::info!("gpu pick resolved {:?}", r);
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

    if !cx.input.state_delta.is_left_mouse_pressing() {
      return;
    }

    let scene = cx.viewer.scene.scene;

    let picker = picker.unwrap();
    let mut hit = None;
    let mut fallback_to_cpu = false;
    if use_gpu_pick && gpu_pick_future.is_none() {
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
      let sms = sms
        .expect_resolve_stage()
        .mark_foreign_key::<SceneModelBelongsToScene>();
      let mut main_scene_models = sms.access_multi(&scene).unwrap();
      let _hit =
        picker.pick_models_nearest(&mut main_scene_models, picker.current_mouse_ray_in_world());
      drop(main_scene_models);

      hit = if let Some(hit) = _hit {
        if enable_hit_debug_log {
          log::info!("cpu picked{:?}", hit);
        }
        hit.1.into()
      } else {
        None
      }
    }

    cx.viewer.scene.selected_target = hit;
  }
}

pub struct PickSceneBlocked;

#[derive(Serialize, Deserialize, Clone)]
pub struct PickScenePersistConfig {
  /// prefer gpu picking for nearest hit query if target platform has correct support
  pub prefer_gpu_picking: bool,
  pub enable_hit_debug_log: bool,
}

impl Default for PickScenePersistConfig {
  fn default() -> Self {
    Self {
      prefer_gpu_picking: true,
      enable_hit_debug_log: false,
    }
  }
}
