use futures::channel::mpsc::UnboundedReceiver;
use rendiation_scene_gltf_loader::*;

use crate::{viewer::use_scene_reader, *};

struct ExportGltfTerminalTask;
impl TerminalTask for ExportGltfTerminalTask {
  type Result = ();
}

pub fn use_enable_gltf_io(cx: &mut ViewerCx) {
  let scene_reader = use_scene_reader(cx);

  let (cx, to_unload) = cx.use_plain_state::<Vec<GltfLoadResult>>();

  if let ViewerCxStage::EventHandling {
    terminal_request, ..
  } = &mut cx.stage
  {
    if let Some(req) = terminal_request.take::<ExportGltfTerminalTask>() {
      let reader = scene_reader.unwrap();
      if let Some(mut dir) = dirs::download_dir() {
        dir.push("gltf_export");

        rendiation_scene_gltf_exporter::build_scene_to_gltf(&reader, &dir, "scene").unwrap();
        req.resolve(());
      } else {
        log::error!("failed to locate the system's default download directory to write file output")
      }
    }
  }
  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    while let Some(gltf_load_info) = to_unload.pop() {
      gltf_load_info.unload(writer);
    }
  }

  let (cx, GltfViewerIO(rev)) = cx.use_state_init(|cx| {
    let (sender, rev) = futures::channel::mpsc::unbounded::<GltfLoadResult>();

    cx.terminal
      .register_command(CMD_LOAD_GLTF, move |ctx, _parameters, tcx| {
        let load_target_node = ctx.scene.root;
        let load_target_scene = ctx.scene.scene;
        let tcx = tcx.clone();
        let sender = sender.clone();

        async move {
          use rfd::AsyncFileDialog;

          let file_handle = AsyncFileDialog::new()
            .add_filter("gltf", &["gltf", "glb"])
            .pick_file()
            .await;

          if let Some(file_handle) = file_handle {

            #[cfg(target_family = "wasm")]
            let file_content = file_handle.read().await;

            #[cfg(target_family = "wasm")]
            let gltf = tcx.worker.spawn_task(move || {
              let _ = trace_span!("parse gltf").entered();
              parse_gltf_from_buffer(&file_content)
            }).await.unwrap();


            #[cfg(not(target_family = "wasm"))]
            let gltf = tcx.worker.spawn_task(move || {
              let _ = trace_span!("parse gltf").entered();
              parse_gltf(file_handle.path())
            }).await.unwrap();


            tcx
              .spawn_main_thread(move || {
                let _ = trace_span!("write gltf into scene").entered();
                let mut writer = SceneWriter::from_global(load_target_scene);

                let load_result = write_gltf_at_node(
                  load_target_node,
                  &mut writer,
                  gltf
                );
                if !load_result.used_but_not_supported_extensions.is_empty() {
                  println!(
                    "warning: gltf load finished but some used(but not required) extensions are not supported: {:#?}",
                    &load_result.used_but_not_supported_extensions
                  );
                }

                sender.unbounded_send(load_result).ok();
              })
              .await;
          }
        }
      });

    cx.terminal
      .register_command(CMD_EXPORT_GLTF, |_ctx, _parameters, tcx| {
        let task = tcx.spawn_event_task::<ExportGltfTerminalTask>();
        async move {
          task.await;
        }
      });

    GltfViewerIO(rev)
  });

  let (cx, current_loaded) = cx.use_plain_state::<Vec<GltfLoadResult>>();

  cx.poll_ctx(|ctx| {
    while let Poll::Ready(Some(result)) = rev.poll_next_unpin(ctx) {
      current_loaded.push(result)
    }
  });

  if let ViewerCxStage::Gui { egui_ctx, global } = &mut cx.stage {
    let opened = global.features.entry("gltf-io").or_insert(false);

    egui::Window::new("Gltf IO")
      .open(opened)
      .default_size((200., 200.))
      .vscroll(true)
      .show(egui_ctx, |ui| {
        if ui.button("export gltf").clicked() {
          cx.viewer
            .terminal
            .buffered_requests
            .push_back(CMD_EXPORT_GLTF.into())
        }

        if ui.button("load gltf").clicked() {
          cx.viewer
            .terminal
            .buffered_requests
            .push_back(CMD_LOAD_GLTF.into())
        }

        let mut to_unload_path = None;
        for result in current_loaded.iter() {
          ui.label(format!("loaded gltf: {:#?}", result.path));
          if ui.button("unload").clicked() {
            to_unload_path = result.path.clone().into();
          }
        }

        if let Some(to_unload_path) = to_unload_path {
          let idx = current_loaded
            .iter()
            .position(|r| r.path == to_unload_path)
            .unwrap();
          let re = current_loaded.swap_remove(idx);
          to_unload.push(re);
        }
      });
  }
}

pub const CMD_EXPORT_GLTF: &str = "export-gltf";
pub const CMD_LOAD_GLTF: &str = "load-gltf";

struct GltfViewerIO(UnboundedReceiver<GltfLoadResult>);
impl CanCleanUpFrom<ViewerDropCx<'_>> for GltfViewerIO {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    cx.terminal.unregister_command(CMD_LOAD_GLTF);
    cx.terminal.unregister_command(CMD_EXPORT_GLTF);
  }
}
