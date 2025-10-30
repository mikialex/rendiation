use crate::*;

pub fn use_pick_scene(cx: &mut ViewerCx) {
  let is_webgl = cx.viewer.rendering.gpu().info().is_webgl();
  let prefer_gpu_pick = !is_webgl && cx.viewer.features_config.pick_scene.prefer_gpu_picking;

  let (cx, gpu_pick_future) =
    cx.use_plain_state::<Option<Box<dyn Future<Output = Option<u32>> + Unpin>>>();

  let picker = use_viewer_scene_model_picker(cx);
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

    let is_request_list_pick = cx
      .input
      .window_state
      .pressed_keys
      .contains(&winit::keyboard::KeyCode::KeyA);

    let picker = picker.unwrap();
    let mut select_target_result = None;
    if let Some(pointer_ctx) = &picker.pointer_ctx {
      let mut use_cpu_pick = false;

      if prefer_gpu_pick && gpu_pick_future.is_none() && !is_request_list_pick {
        if let Some(view_renderer) = cx.viewer.rendering.views.get_mut(&pointer_ctx.viewport_id) {
          if let Some(render_size) = view_renderer.picker.last_id_buffer_size() {
            let point = pointer_ctx.normalized_position * Vec2::from(render_size.into_f32());
            let point = point.map(|v| v.floor() as usize);
            if let Some(f) = view_renderer.picker.pick_point_at((point.x, point.y)) {
              *gpu_pick_future = Some(f);
            }
          } else {
            use_cpu_pick = true;
          }
        } else {
          use_cpu_pick = true;
        }
      } else {
        use_cpu_pick = true;
      }

      if use_cpu_pick {
        let sms = sms
          .expect_resolve_stage()
          .mark_foreign_key::<SceneModelBelongsToScene>();
        let mut main_scene_models = sms.access_multi(&scene).unwrap();

        if is_request_list_pick {
          let (results, result_ids) =
            picker.pick_models_all(&mut main_scene_models, pointer_ctx.world_ray);
          if enable_hit_debug_log {
            log::info!("cpu picked list {:#?}, ids: {:#?}", results, result_ids);
          }
        } else {
          let _hit = picker.pick_models_nearest(&mut main_scene_models, pointer_ctx.world_ray);
          drop(main_scene_models);

          select_target_result = if let Some(hit) = _hit {
            if enable_hit_debug_log {
              log::info!("cpu picked {:#?}", hit);
            }
            hit.1.into()
          } else {
            None
          }
        }
      }
    }

    cx.viewer.scene.selected_target = select_target_result;
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
      enable_hit_debug_log: true,
    }
  }
}
