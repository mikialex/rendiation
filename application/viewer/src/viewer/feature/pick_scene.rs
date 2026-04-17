use crate::*;

pub fn inject_picker(cx: &mut ViewerCx, f: impl FnOnce(&mut ViewerCx)) {
  let mut picker = use_viewer_scene_model_picker(cx);

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    let picker = picker.as_mut().unwrap();
    unsafe { cx.dyn_cx.register_cx(picker) };
  }

  f(cx);

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    unsafe {
      cx.dyn_cx.unregister_cx::<ViewerPickerWithCtx>();
    }
  }
}

pub fn use_pick_scene(cx: &mut ViewerCx) {
  let is_webgl = cx.viewer.rendering.gpu().info().is_webgl();
  let prefer_gpu_pick = !is_webgl && cx.viewer.features_config.pick_scene.prefer_gpu_picking;

  let (cx, gpu_pick_future) =
    cx.use_plain_state::<Option<Box<dyn Future<Output = Option<u32>> + Unpin>>>();

  let (cx, range_state) = cx.use_plain_state::<Option<(Vec2<f32>, Vec2<f32>)>>();

  if let ViewerCxStage::Gui {
    egui_ctx, global, ..
  } = &mut cx.stage
  {
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

    // draw ui rect
    if let Some((start, end)) = range_state {
      egui::Area::new(egui::Id::new("range_pick"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .show(egui_ctx, |ui| {
          let width = (start.x - end.x).abs();
          let height = (start.y - end.y).abs();
          let x = start.x.min(end.x);
          let y = start.y.min(end.y);

          ui.painter().rect_filled(
            egui::Rect::from_min_size(egui::pos2(x, y), egui::Vec2::new(width, height)),
            0.0,
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 100),
          );
        });
    }
  }

  let enable_hit_debug_log = cx.viewer.features_config.pick_scene.enable_hit_debug_log;

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    if let Some(f) = gpu_pick_future {
      noop_ctx!(ctx);
      if let Poll::Ready(r) = f.poll_unpin(ctx) {
        if enable_hit_debug_log {
          log::info!("gpu pick resolved {:?}", r);
        }

        cx.active_surface_content.selected_model.clear();
        if let Some(hit_entity_idx) = r {
          // skip the background
          if hit_entity_idx != u32::MAX {
            if let Some(hit) = global_entity_of::<SceneModelEntity>()
              .entity_reader()
              .reconstruct_handle_by_idx(hit_entity_idx as usize)
            {
              cx.active_surface_content.selected_model.add_select(hit);
            }
          }
        }

        *gpu_pick_future = None;
      }
    }

    if cx.input.state_delta.is_left_mouse_releasing() {
      if let Some((a, b)) = range_state.take() {
        let a = a * cx.active_surface_content.device_pixel_ratio;
        let b = b * cx.active_surface_content.device_pixel_ratio;

        log::info!("end range {:?}", (a, b));
        access_cx!(cx.dyn_cx, picker, ViewerPickerWithCtx);
        let scene = cx.active_surface_content.scene;

        let (viewport, normalized_a) =
          find_top_hit(cx.active_surface_content.viewports.iter(), a.into()).unwrap();
        let (viewport_, normalized_b) =
          find_top_hit(cx.active_surface_content.viewports.iter(), b.into()).unwrap();
        assert_eq!(viewport.id, viewport_.id);
        let a = Vec2::from(normalized_a);
        let b = Vec2::from(normalized_b);

        let min = a.min(b);
        let max = a.max(b);

        dbg!(&min);
        dbg!(&max);

        let ndc_arr = [
          min.x as f64,
          max.x as f64,
          min.y as f64,
          max.y as f64,
          0.0,
          1.0,
        ];

        let camera = viewport.camera;
        let camera_trans = picker
          .camera_transforms
          .access(camera.raw_handle_ref())
          .unwrap();

        let ndc = cx.viewer.ndc();
        let mat =
          ndc.transform_into_opengl_standard_ndc().into_f64() * camera_trans.view_projection;
        let frustum = Frustum::new_from_matrix_ndc(mat, &ndc_arr);

        let r = picker.pick_range(scene, frustum, ObjectTestPolicy::Intersect);
        log::info!("range pick results {:?}", r);
        for m in r {
          cx.active_surface_content.selected_model.add_select(m);
        }
      }
      *range_state = None;
    }
    if cx.input.state_delta.mouse_position_change {
      if let Some((_start, end)) = range_state {
        let position = cx.input.window_state.mouse_position_in_logic_pixel();
        *end = position.into();
      }
    }

    if range_state.is_some() {
      cx.dyn_cx.message.put(CameraControlBlocked);
    }

    if cx.dyn_cx.message.get::<PickSceneBlocked>().is_some() {
      return;
    }

    if !cx.input.state_delta.is_left_mouse_pressing() {
      return;
    }

    let pressed_keys = &cx.input.window_state.pressed_keys;
    let is_start_range_pick = pressed_keys.contains(&winit::keyboard::KeyCode::KeyQ);

    if is_start_range_pick {
      log::info!("start range pick");
      let position = cx.input.window_state.mouse_position_in_logic_pixel();
      *range_state = Some((position.into(), position.into()))
      //
    } else {
      let is_request_list_pick = pressed_keys.contains(&winit::keyboard::KeyCode::KeyA);

      let scene = cx.active_surface_content.scene;

      access_cx!(cx.dyn_cx, picker, ViewerPickerWithCtx);

      if let Some(pointer_ctx) = &picker.pointer_ctx {
        let mut use_cpu_pick = false;

        if prefer_gpu_pick && gpu_pick_future.is_none() && !is_request_list_pick {
          let surface_views = &mut cx.viewer.rendering.surface_views;
          let surface_view = surface_views.get_mut(&cx.surface_id).unwrap();
          if let Some(view_renderer) = surface_view.get_mut(&pointer_ctx.viewport_id) {
            if let Some(render_size) = view_renderer.picker.last_id_buffer_size() {
              let point = (pointer_ctx.normalized_position * Vec2::new(0.5, -0.5)
                + Vec2::new(0.5, 0.5))
                * Vec2::from(render_size.into_f32());
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
          if is_request_list_pick {
            let (results, result_ids) = picker.pick_models_list_all(pointer_ctx.world_ray, scene);
            if enable_hit_debug_log {
              log::info!("cpu picked list {:#?}, ids: {:#?}", results, result_ids);
            }
          } else {
            let _hit = picker.pick_model_nearest_all(pointer_ctx.world_ray, scene);

            cx.active_surface_content.selected_model.clear();
            if let Some(hit) = _hit {
              if enable_hit_debug_log {
                log::info!("cpu picked {:#?}", hit);
              }
              cx.active_surface_content.selected_model.add_select(hit.1);
            }
          }
        }
      }
    }
  }
}

pub struct PickSceneBlocked;
