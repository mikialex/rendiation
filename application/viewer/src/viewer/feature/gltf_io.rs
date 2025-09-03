use crate::{viewer::use_scene_reader, *};

struct ExportGltfTerminalTask;
impl TerminalTask for ExportGltfTerminalTask {
  type Result = ();
}

pub fn use_enable_gltf_io(cx: &mut ViewerCx) {
  let scene_reader = use_scene_reader(cx);

  if let ViewerCxStage::EventHandling {
    terminal_request, ..
  } = &mut cx.stage
  {
    if let Some(req) = terminal_request.take::<ExportGltfTerminalTask>() {
      let reader = scene_reader.unwrap();
      if let Some(mut dir) = dirs::download_dir() {
        dir.push("gltf_export");

        rendiation_scene_gltf_exporter::build_scene_to_gltf(
          reader,
          cx.viewer.scene.root,
          &dir,
          "scene",
        )
        .unwrap();
        req.resolve(());
      } else {
        log::error!("failed to locate the system's default download directory to write file output")
      }
    }
  }

  cx.use_state_init(|cx| {
      cx.terminal.register_command("load-gltf", |ctx, _parameters, tcx| {
    let load_target_node = ctx.scene.root;
    let load_target_scene = ctx.scene.scene;
    let tcx = tcx.clone();

    async move {
      use rfd::AsyncFileDialog;

      let file_handle = AsyncFileDialog::new()
        .add_filter("gltf", &["gltf", "glb"])
        .pick_file()
        .await;

      if let Some(file_handle) = file_handle {
      let gltf = tcx.worker.spawn_task(move || {
        let _ = trace_span!("parse gltf").entered();
        rendiation_scene_gltf_loader::parse_gltf(file_handle.path())
      }).await.unwrap();

        tcx
          .spawn_main_thread(move || {
            let _ = trace_span!("write gltf into scene").entered();
            let mut writer = SceneWriter::from_global(load_target_scene);

            let load_result = rendiation_scene_gltf_loader::write_gltf_at_node(
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
          })
          .await;
      }
    }
  });


  cx.terminal.register_command("export-gltf", |_ctx, _parameters, tcx| {
    let task =  tcx.spawn_event_task::<ExportGltfTerminalTask>();
    async move{
      task.await;
    }
  });

  GltfViewerIO
  });
}

struct GltfViewerIO;
impl CanCleanUpFrom<ViewerDropCx<'_>> for GltfViewerIO {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    cx.terminal.unregister_command("load-gltf");
    cx.terminal.unregister_command("export-gltf");
  }
}
