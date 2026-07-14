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
  let prefer_gpu_pick = !is_webgl && cx.app_features.pick_scene.prefer_gpu_picking;

  let (cx, gpu_pick_future) =
    cx.use_plain_state::<Option<Box<dyn Future<Output = Option<u32>> + Unpin>>>();

  let (cx, range_state) = cx.use_plain_state::<Option<(Vec2<f32>, Vec2<f32>)>>();

  let (cx, request_bvh_debug) = cx.use_plain_state::<bool>();

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
          &mut cx.app_features.pick_scene.prefer_gpu_picking,
          "prefer gpu pick",
        );
        ui.checkbox(
          &mut cx.app_features.pick_scene.enable_hit_debug_log,
          "enable pick log",
        );
        ui.checkbox(
          &mut cx.app_features.pick_scene.range_query_contains,
          "use contain test for range test",
        );
        ui.checkbox(
          &mut cx.app_features.pick_scene.precise_intersection_test,
          "use precise intersection test",
        );

        if ui.button("debug scene bvh").clicked() {
          *request_bvh_debug = true;
        }
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

  let enable_hit_debug_log = cx.app_features.pick_scene.enable_hit_debug_log;
  let use_contain_for_range_test = cx.app_features.pick_scene.range_query_contains;
  let precise_intersection_test = cx.app_features.pick_scene.precise_intersection_test;

  let (cx, request_bvh_debug_to_write) = cx.use_plain_state::<Option<Vec<WideLineVertex>>>();
  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    if let Some(vertices) = request_bvh_debug_to_write.take() {
      let buffer = ExternalRefPtr::new(bytemuck::cast_slice(&vertices).to_vec());

      let wide_line_model = global_entity_of::<WideLineModelEntity>()
        .entity_writer()
        .new_entity(|w| {
          w.write::<WideLineWidth>(&2.0)
            .write::<WideLineMeshBuffer>(&buffer)
        });

      let scene = cx.default_scene.scene.some_handle();
      let node = writer.create_root_child();
      writer.model_writer.new_entity(|w| {
        w.write::<SceneModelWideLineRenderPayload>(&wide_line_model.some_handle())
          .write::<SceneModelBelongsToScene>(&scene)
          .write::<SceneModelRefNode>(&node.some_handle())
      });
    }
  }

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    if let Some(f) = gpu_pick_future {
      noop_ctx!(ctx);
      if let Poll::Ready(r) = f.poll_unpin(ctx) {
        if enable_hit_debug_log {
          log::info!("gpu pick resolved {:?}", r);
        }

        cx.viewer.selection.selected_model.clear();
        if let Some(hit_entity_idx) = r {
          // skip the background
          if hit_entity_idx != u32::MAX {
            if let Some(hit) = global_entity_of::<SceneModelEntity>()
              .entity_reader()
              .reconstruct_handle_by_idx(hit_entity_idx as usize)
            {
              cx.viewer.selection.selected_model.add_select(hit);
            }
          }
        }

        *gpu_pick_future = None;
      }
    }

    if *request_bvh_debug {
      access_cx!(cx.dyn_cx, picker, ViewerPickerWithCtx);
      *request_bvh_debug = false;
      if let Some(bvh_line_buffer) = picker.picker_impl.debug_bvh(cx.default_scene.scene) {
        let max_depth = bvh_line_buffer.len().saturating_sub(1);
        let vertices: Vec<WideLineVertex> = bvh_line_buffer
          .iter()
          .enumerate()
          .flat_map(|(depth, lines)| {
            let t = if max_depth > 0 {
              depth as f32 / max_depth as f32
            } else {
              0.0
            };
            // red (0) -> green (0.5) -> blue (1)
            let color = Vec4::new(
              (1.0 - t * 2.0).max(0.0),
              1.0 - (t * 2.0 - 1.0).abs(),
              (t * 2.0 - 1.0).max(0.0),
              1.0,
            );
            lines.iter().map(move |(start, end)| WideLineVertex {
              start: *start,
              end: *end,
              color,
            })
          })
          .collect();
        *request_bvh_debug_to_write = Some(vertices);
      }
    }

    if cx.input.state_delta.is_left_mouse_releasing() {
      if let Some((a, b)) = range_state.take() {
        log::info!("end range {:?}", (a, b));

        access_cx!(cx.dyn_cx, picker, ViewerPickerWithCtx);

        if let Some((frustum, scene)) = create_range_pick_frustum(
          a,
          b,
          cx.active_surface_content,
          &picker.picker_impl,
          precise_intersection_test,
          0.,
        ) {
          let r = measure_and_log_time("cpu pick range", || {
            picker.pick_range(
              scene,
              &frustum,
              if use_contain_for_range_test {
                ObjectTestPolicy::Contains
              } else {
                ObjectTestPolicy::Intersect
              },
            )
          });
          log::info!("range pick results {:?}", r);
          for m in r {
            cx.viewer.selection.selected_model.add_select(m);
          }
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

      access_cx!(cx.dyn_cx, picker, ViewerPickerWithCtx);

      if let Some((pointer_ctx, scene)) = &picker.pointer_ctx {
        let mut use_cpu_pick = false;

        // todo watch prefer_gpu_pick changed
        let surface_views = &mut cx.viewer.rendering.surface_views;
        let surface_view = surface_views.get_mut(&cx.surface_id).unwrap();
        if let Some(view_renderer) = surface_view.get_mut(&pointer_ctx.viewport_id) {
          // this will take effect in next frame but it's ok i assume
          view_renderer.enable_gpu_pick_id_write = prefer_gpu_pick;
        }

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
            let (results, result_ids) = measure_and_log_time("cpu pick list", || {
              picker.pick_models_list_all(pointer_ctx.world_ray, *scene)
            });
            if enable_hit_debug_log {
              log::info!(
                "cpu picked list result {:#?}, ids: {:#?}",
                results,
                result_ids
              );
            }
          } else {
            let _hit = measure_and_log_time("cpu pick nearest", || {
              picker.pick_model_nearest_all(pointer_ctx.world_ray, *scene)
            });

            cx.viewer.selection.selected_model.clear();
            if let Some(hit) = _hit {
              if enable_hit_debug_log {
                log::info!("cpu pick nearest result {:#?}", hit);
              }
              cx.viewer.selection.selected_model.add_select(hit.1);
            }
          }
        }
      }
    }
  }
}

pub struct PickSceneBlocked;

fn measure_and_log_time<R>(label: &str, f: impl FnOnce() -> R) -> R {
  let start = std::time::Instant::now();
  let r = f();
  log::info!("{} time: {}ms", label, start.elapsed().as_millis());
  r
}
