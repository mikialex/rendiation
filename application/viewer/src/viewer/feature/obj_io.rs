#![allow(unused_variables)]

use crate::*;

pub fn use_enable_obj_io(cx: &mut ViewerCx) {
  if let ViewerCxStage::Gui { egui_ctx, global } = &mut cx.stage {
    let opened = global.features.entry("obj-io").or_insert(false);

    egui::Window::new("Obj(wavefront) IO")
      .open(opened)
      .vscroll(true)
      .show(egui_ctx, |ui| {
        if ui.button("load obj").clicked() {
          cx.viewer
            .terminal
            .buffered_requests
            .push_back(CMD_LOAD_WAVEFRONT_OBJ.into())
        }
      });
  }

  cx.use_state_init(|cx| {
    cx.terminal
      .register_command(CMD_LOAD_WAVEFRONT_OBJ, |ctx, _parameters, tcx| {
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

                #[cfg(not(target_family = "wasm"))]
                rendiation_scene_obj_loader::load_obj(
                  file_handle.path(),
                  load_target_node,
                  default_mat,
                  &mut writer,
                )
                .unwrap();

                #[cfg(target_family = "wasm")]
                todo!()
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
    cx.terminal.unregister_command(CMD_LOAD_WAVEFRONT_OBJ);
  }
}

pub const CMD_LOAD_WAVEFRONT_OBJ: &str = "load-obj";
