use database::global_database;
use database_tracing::*;

use crate::viewer::*;

pub const CMD_CONVERT_TRACE: &str = "convert-trace";

struct TraceIOState;

impl CanCleanUpFrom<ViewerDropCx<'_>> for TraceIOState {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    cx.terminal.unregister_command(CMD_CONVERT_TRACE);
  }
}

pub fn use_enable_trace_io(cx: &mut ViewerCx) {
  let _state = cx.use_state_init(|cx| {
    cx.terminal
      .register_command(CMD_CONVERT_TRACE, |_ctx, _parameters, tcx| {
        let tcx = tcx.clone();
        async move {
          let file_handle = rfd::AsyncFileDialog::new()
            .add_filter("trace", &["bin"])
            .pick_file()
            .await;

          let Some(file_handle) = file_handle else {
            return;
          };

          let input = file_handle.path().to_path_buf();
          let output_dir = file_handle
            .path()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
          let output_path = output_dir.join("trace.txt");

          let db = global_database();

          let result = tcx
            .worker
            .spawn_task(move || {
              let mut output = std::fs::File::create(&output_path)
                .map_err(|e| format!("failed to create output: {e}"))?;

              trace_to_text::<()>(&input, &mut output, Some(&db), 1024)
                .map_err(|e| format!("conversion failed: {e}"))?;

              Ok::<_, String>(output_path)
            })
            .await;

          match result {
            Ok(path) => log::info!("trace converted to {}", path.display()),
            Err(e) => log::error!("{e}"),
          }
        }
      });

    TraceIOState
  });

  if let ViewerCxStage::Gui {
    egui_ctx, global, ..
  } = &mut cx.stage
  {
    let opened = global.features.entry("trace-io").or_insert(false);

    egui::Window::new("Trace Converter")
      .open(opened)
      .default_size((200., 80.))
      .show(egui_ctx, |ui| {
        if ui.button("convert trace.bin to text").clicked() {
          cx.viewer
            .terminal
            .buffered_requests
            .push_back(CMD_CONVERT_TRACE.into());
        }
      });
  }
}
