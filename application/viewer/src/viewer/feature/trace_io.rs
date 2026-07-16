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

struct TraceReplayState {
  state: ReplayState,
  file_name: String,
}

pub fn use_enable_trace_io(cx: &mut ViewerCx) {
  let (cx, replay) = cx.use_plain_state::<Option<TraceReplayState>>();
  let db = global_database();

  let _state = cx.use_state_init(|cx| {
    cx.terminal
      .register_command(CMD_CONVERT_TRACE, |_ctx, _parameters, tcx| {
        let tcx = tcx.clone();
        let db = global_database();
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
          let result = tcx
            .worker
            .spawn_task(move || {
              let mut output = std::fs::File::create(&output_path)
                .map_err(|e| format!("failed to create output: {e}"))?;
              trace_to_text::<crate::ViewerTracingEvent>(&input, &mut output, Some(&db), 1024)
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
    egui_ui, global, ..
  } = &mut cx.stage
  {
    let opened = global.features.entry("trace-io").or_insert(false);

    egui::Window::new("Trace IO")
      .open(opened)
      .default_size((500., 400.))
      .show(egui_ui, |ui| {
        ui.collapsing("Convert", |ui| {
          if ui.button("select .bin and convert to .txt").clicked() {
            cx.viewer
              .terminal
              .buffered_requests
              .push_back(CMD_CONVERT_TRACE.into());
          }
        });

        ui.collapsing("Replay", |ui| {
          if ui.button("load trace.bin for replay").clicked() {
            match rfd::FileDialog::new()
              .add_filter("trace", &["bin"])
              .pick_file()
            {
              Some(path) => {
                let file_name = path
                  .file_name()
                  .map(|n| n.to_string_lossy().to_string())
                  .unwrap_or_else(|| "?".into());
                match load_replay::<crate::ViewerTracingEvent>(&path) {
                  Ok(state) => {
                    let count = state.records.len();
                    *replay = Some(TraceReplayState { state, file_name });
                    log::info!("loaded {} records", count);
                  }
                  Err(e) => {
                    log::error!("failed to load replay: {e}");
                  }
                }
              }
              None => {}
            }
          }

          if let Some(ref mut rs) = replay.as_mut() {
            let total = rs.state.records.len();
            let pos = rs.state.position;
            ui.label(format!("{} — {}/{} records", rs.file_name, pos, total));

            ui.horizontal(|ui| {
              if ui.button(">").clicked() {
                step_forward(&mut rs.state, &db);
              }
              if ui.button(">|").clicked() {
                restart_and_run_to(&mut rs.state, &db, total);
              }
            });

            let table = egui_extras::TableBuilder::new(ui)
              .striped(true)
              .column(egui_extras::Column::auto().resizable(true))
              .column(egui_extras::Column::remainder().clip(true))
              .max_scroll_height(300.)
              .cell_layout(egui::Layout::left_to_right(egui::Align::Center));

            table.body(|body| {
              body.rows(20.0, total, |mut row| {
                let idx = row.index();
                row.set_selected(idx == pos);
                let record = &rs.state.records[idx];
                row.col(|ui| {
                  ui.label(format!("#{}", idx));
                });
                row.col(|ui| {
                  ui.label(&record.summary);
                });
                if row.response().clicked() {
                  restart_and_run_to(&mut rs.state, &db, idx);
                }
              });
            });
          }
        });
      });
  }
}
