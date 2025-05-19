use crate::*;

pub fn use_enable_gltf_io(cx: &mut ViewerCx) {
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
        tcx
          .spawn_main_thread(move || {
            let mut writer = SceneWriter::from_global(load_target_scene);

            let load_result = rendiation_scene_gltf_loader::load_gltf(
              file_handle.path(),
              load_target_node,
              &mut writer,
            )
            .unwrap();
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


  cx.terminal.register_command("export-gltf", |ctx, _parameters, tcx| {
    let derive_update = ctx.derive.poll_update();
    let node_children = derive_update.node_children;
    let mesh_ref_vertex = derive_update.mesh_vertex_ref;
    let sm_ref_s = derive_update.sm_to_s;

    let export_root_node = ctx.scene.root;
    let export_scene = ctx.scene.scene;

    let tcx = tcx.clone();

    async move {
      if let Some(mut dir) = dirs::download_dir() {
        dir.push("gltf_export");

        tcx
          .spawn_main_thread(move || {
            let reader =
              SceneReader::new_from_global(export_scene, mesh_ref_vertex, node_children, sm_ref_s);

            rendiation_scene_gltf_exporter::build_scene_to_gltf(
              reader,
              export_root_node,
              &dir,
              "scene",
            )
            .unwrap();
          })
          .await;
      } else {
        log::error!("failed to locate the system's default download directory to write file output")
      }
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
