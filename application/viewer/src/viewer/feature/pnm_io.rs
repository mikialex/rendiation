use std::path::PathBuf;

use futures::channel::mpsc::UnboundedReceiver;
use parametric_normalized_model::ParametricNormalizedModel;
use winit::event::ElementState;

use crate::*;

pub const CMD_LOAD_PNM: &str = "load-pnm";

/// A `.pnm` file path loaded at startup, takes precedence over
/// `ViewerAppFeaturesConfig::auto_load_pnm`.
pub const PNM_AUTO_LOAD_ENV: &str = "RENDIATION_VIEWER_PNM";

/// Loads parametric models (`.pnm`) into `Viewer3dRenderingCtx::parametric` by drag and drop, the
/// file dialog of the "Parametric Model" window or the `load-pnm [path]` command, and shows the
/// parametric renderer settings. Loading replaces the current model and focuses the camera on it.
pub fn use_enable_pnm_io(cx: &mut ViewerCx) {
  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    for e in &cx.input.accumulate_events {
      if let Event::WindowEvent { event, .. } = e
        && let WindowEvent::DroppedFile(file) = event
        && file.extension().is_some_and(|ext| ext == "pnm")
      {
        cx.viewer.terminal.buffered_requests.push_back(format!(
          "{} {}",
          CMD_LOAD_PNM,
          file.to_string_lossy()
        ));
      }
    }
  }

  let (cx, PnmViewerIO(receiver)) = cx.use_state_init(|cx| {
    let (sender, receiver) = futures::channel::mpsc::unbounded::<ParametricModelSource>();

    cx.terminal
      .register_command(CMD_LOAD_PNM, move |_ctx, parameters, tcx| {
        let tcx = tcx.clone();
        let sender = sender.clone();
        // the path may contain spaces, which split it into several parameters
        let file_path = (parameters.len() > 1).then(|| PathBuf::from(parameters[1..].join(" ")));

        async move {
          let worker = tcx.worker.clone();
          let source = if let Some(file_path) = file_path {
            let source = worker.spawn_task(move || {
              let _ = trace_span!("load pnm").entered();
              let name = file_path.file_name().unwrap_or_default().to_string_lossy();
              ParametricNormalizedModel::load_pnm(&file_path)
                .map(|model| ParametricModelSource::new(model, name.into_owned()))
                .map_err(|e| format!("failed to load pnm {}: {e:?}", file_path.display()))
            });
            Some(source.await)
          } else {
            use rfd::AsyncFileDialog;
            let file_handle = AsyncFileDialog::new()
              .add_filter("parametric normalized model", &["pnm"])
              .pick_file()
              .await;

            if let Some(file_handle) = file_handle {
              let name = file_handle.file_name();
              let content = file_handle.read().await;
              let source = worker.spawn_task(move || {
                let _ = trace_span!("parse pnm").entered();
                ParametricNormalizedModel::read_pnm(content.as_slice())
                  .map(|model| ParametricModelSource::new(model, name.clone()))
                  .map_err(|e| format!("failed to parse pnm {name}: {e:?}"))
              });
              Some(source.await)
            } else {
              None
            }
          };

          match source {
            Some(Ok(source)) => {
              sender.unbounded_send(source).ok();
            }
            Some(Err(e)) => log::error!("{e}"),
            None => {}
          }
        }
      });

    let auto_load = std::env::var(PNM_AUTO_LOAD_ENV)
      .ok()
      .or_else(|| cx.app_features.auto_load_pnm.clone());
    if let Some(path) = auto_load {
      log::info!("auto load parametric model {path}");
      cx.terminal
        .buffered_requests
        .push_back(format!("{CMD_LOAD_PNM} {path}"));
    }

    PnmViewerIO(receiver)
  });

  let mut loaded = None;
  cx.poll_ctx(|ctx| {
    while let Poll::Ready(Some(source)) = receiver.poll_next_unpin(ctx) {
      loaded = Some(source);
    }
  });

  let (cx, focus_requested) = cx.use_plain_state::<bool>();
  let (cx, open_window_requested) = cx.use_plain_state::<bool>();
  if let Some(source) = loaded {
    cx.viewer.rendering.parametric.set_model(source);
    cx.viewer.rendering_root.notify_change();
    *focus_requested = true;
    *open_window_requested = true;
  }

  use_focus_camera_on_parametric_model(cx, focus_requested);

  if let ViewerCxStage::Gui {
    egui_ui, global, ..
  } = &mut cx.stage
  {
    let opened = global.features.entry("parametric-model").or_insert(false);
    *opened |= std::mem::take(open_window_requested);

    let mut changed = false;
    egui::Window::new("Parametric Model")
      .open(opened)
      .default_size((320., 480.))
      .vscroll(true)
      .show(egui_ui, |ui| {
        if ui.button("load pnm").clicked() {
          cx.viewer
            .terminal
            .buffered_requests
            .push_back(CMD_LOAD_PNM.to_string());
        }

        let parametric = &mut cx.viewer.rendering.parametric;
        if let Some(source) = parametric.model() {
          let model = &source.model;
          ui.label(format!("model: {}", source.name));
          ui.label(format!(
            "{} patches ({} untrimmed), {} instances, unit: {}",
            model.patches.len(),
            source.untrimmed_patch_count,
            model.instances.len(),
            model
              .length_unit_mm
              .map(|mm| format!("{mm} mm"))
              .unwrap_or("unknown".into()),
          ));
          let size = source.bounds.max - source.bounds.min;
          ui.label(format!(
            "model space size: {:.3} x {:.3} x {:.3}",
            size.x, size.y, size.z
          ));
          ui.horizontal(|ui| {
            if ui.button("focus camera on model").clicked() {
              *focus_requested = true;
            }
            if ui.button("unload").clicked() {
              parametric.clear_model();
              changed = true;
            }
          });
        } else {
          ui.label("no model loaded, drop a .pnm file here or use the load button");
        }

        ui.separator();
        changed |= parametric.egui(ui);
      });

    if changed {
      cx.viewer.rendering_root.notify_change();
    }
  }
}

/// Fit the first viewport camera to the parametric model bounds when requested.
fn use_focus_camera_on_parametric_model(cx: &mut ViewerCx, focus_requested: &mut bool) {
  let world_mat = use_global_node_world_mat_view(cx).use_assure_result(cx);

  let key_pressed = cx
    .input
    .state_delta
    .key_state_changes
    .get(&winit::keyboard::KeyCode::KeyP)
    == Some(&ElementState::Pressed);

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    if !std::mem::take(focus_requested) && !key_pressed {
      return;
    }
    let Some(bounds) = cx.viewer.rendering.parametric.world_bounds() else {
      return;
    };
    let Some(viewport) = cx.active_surface_content.viewports.first() else {
      return;
    };
    let world_mat = world_mat.expect_resolve_stage().mark_entity_type();
    if let Some(camera_world) = world_mat.access(&viewport.camera_node)
      && let Some(proj) = read_common_proj_from_db(viewport.camera)
      && let Some(action) = fit_camera_view(proj, camera_world, bounds)
    {
      cx.dyn_cx.message.put(action);
    }
  }
}

struct PnmViewerIO(UnboundedReceiver<ParametricModelSource>);
impl CanCleanUpFrom<ViewerDropCx<'_>> for PnmViewerIO {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    cx.terminal.unregister_command(CMD_LOAD_PNM);
  }
}
