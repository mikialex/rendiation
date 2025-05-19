use crate::*;

pub fn use_enable_obj_io(cx: &mut ViewerCx) {
  cx.use_state_init(|cx| {
    cx.terminal
      .register_command("load-obj", |ctx, _parameters, tcx| {
        let load_target_node = ctx.scene.root;
        let load_target_scene = ctx.scene.scene;
        let tcx = tcx.clone();

        async move {
          use rfd::AsyncFileDialog;

          let file_handle = AsyncFileDialog::new()
            .add_filter("obj", &["obj"])
            .pick_file()
            .await;

          if let Some(file_handle) = file_handle {
            tcx
              .spawn_main_thread(move || {
                let mut writer = SceneWriter::from_global(load_target_scene);
                let default_mat = writer.pbr_sg_mat_writer.new_entity();

                rendiation_scene_obj_loader::load_obj(
                  file_handle.path(),
                  load_target_node,
                  default_mat,
                  &mut writer,
                )
                .unwrap();
              })
              .await;
          }
        }
      });

    ObjViewerIO
  });
}

struct ObjViewerIO;
impl CanCleanUpFrom<ViewerDropCx<'_>> for ObjViewerIO {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    cx.terminal.unregister_command("load-obj");
  }
}
