use std::any::TypeId;

use egui::Response;
use egui_extras::{Column, TableBuilder};
use egui_wgpu::wgpu::naga::FastHashMap;

use crate::*;

pub struct DBInspector {
  inspector: DataDebugger,
  visit_history: Vec<Option<EntityId>>,
  current: usize,
}

impl Default for DBInspector {
  fn default() -> Self {
    let mut inspector = DataDebugger::default();

    inspector
      .register::<Option<RawEntityHandle>>()
      .register::<f32>()
      .register::<Vec2<f32>>()
      .register::<Vec3<f32>>()
      .register::<Vec4<f32>>()
      .register::<Mat4<f64>>();

    Self {
      inspector,
      visit_history: vec![None],
      current: 0,
    }
  }
}

impl DBInspector {
  pub fn current(&self) -> Option<EntityId> {
    self.visit_history[self.current]
  }
  pub fn goto(&mut self, target: Option<EntityId>) {
    self.visit_history.truncate(self.current + 1); // drop history after current
    self.visit_history.push(target);
    self.current += 1;
  }

  pub fn can_go_back(&self) -> bool {
    self.current > 0
  }
  pub fn can_go_forward(&self) -> bool {
    self.current < self.visit_history.len() - 1
  }

  pub fn has_history(&self) -> bool {
    self.can_go_back() || self.can_go_forward()
  }

  pub fn clear_history(&mut self) {
    self.current = 0;
    self.visit_history = vec![None];
  }

  pub fn go_back(&mut self) {
    self.current = self.current.saturating_sub(1);
  }
  pub fn go_forward(&mut self) {
    self.current += 1;
  }
}

pub fn egui_db_gui(ui: &egui::Context, state: &mut DBInspector, opened: &mut bool) {
  egui::Window::new("Database Inspector")
    .open(opened)
    .min_width(500.0)
    .max_width(700.0)
    .min_height(400.0)
    .max_height(1000.0)
    .default_width(800.0)
    .default_height(400.)
    .resizable(true)
    .movable(true)
    .scroll([true, true])
    .show(ui, |ui| {
      let mut back_to_all_table_view = false;
      ui.horizontal_wrapped(|ui| {
        if state.current().is_some() {
          back_to_all_table_view = ui.button("View all tables in DB").clicked();
        }
        if state.can_go_back() && ui.button("Back").clicked() {
          state.go_back();
        }
        if state.can_go_forward() && ui.button("Previous").clicked() {
          state.go_forward();
        }
        if state.has_history() && ui.button("Clear visit history").clicked() {
          state.clear_history();
        }
      });
      ui.separator();

      if let Some(visiting_entity) = &state.current() {
        selected_table(ui, state, *visiting_entity);
      } else {
        all_tables(ui, state);
      }

      if back_to_all_table_view {
        state.goto(None);
      }
    });
}

fn selected_table(ui: &mut egui::Ui, state: &mut DBInspector, e_id: EntityId) {
  let db = global_database();
  db.access_table_dyn(e_id, |tb| {
    ui.heading(tb.name());

    let table = TableBuilder::new(ui)
      .striped(true)
      .column(Column::auto())
      .columns(
        Column::auto()
          .resizable(true)
          .at_least(100.)
          .at_most(300.)
          .clip(true),
        tb.component_count(),
      )
      .max_scroll_height(900.)
      .cell_layout(egui::Layout::left_to_right(egui::Align::Center));

    tb.access_components(|coms| {
      table
        .header(20.0, |mut header| {
          header.col(|ui| {
            ui.strong("entity id");
          });
          coms.values().for_each(|com| {
            header.col(|ui| {
              let label = ui.strong(com.short_name.as_str());

              if let Some(f) = com.as_foreign_key {
                label.highlight().on_hover_ui(|ui| {
                  let name = db.access_table_dyn(f, |tb| tb.name().to_string());
                  if ui.link(name).clicked() {
                    state.goto(Some(f));
                  }
                });
              }
            });
          })
        })
        .body(|body| {
          body.rows(20.0, tb.entity_capacity(), |mut row| {
            let idx = row.index();
            row.col(|ui| {
              ui.label(idx.to_string());
            });
            coms.values().for_each(|com| {
              row.col(|ui| {
                let reader = com.read_untyped();
                if let Some((data_ptr, data)) =
                  reader.get_without_generation_check_dyn_data_type(idx as u32)
                {
                  let fallback_debug = data.debug_value();

                  state
                    .inspector
                    .ui(&com.data_meta.data_type_id, data_ptr, ui, fallback_debug);
                } else {
                  ui.weak("not exist");
                }
              });
            })
            //
          })
        });
    });
  })
}

#[derive(Default)]
struct DataDebugger {
  inline_ui: FastHashMap<TypeId, fn(*const (), &mut egui::Ui) -> Response>,
  hover_inspect_ui: FastHashMap<TypeId, fn(*const (), &mut egui::Ui)>,
}

impl DataDebugger {
  pub fn register<T: EGUIDataView + 'static>(&mut self) -> &mut Self {
    self.inline_ui.insert(TypeId::of::<T>(), |data, ui| {
      let data = unsafe { &*(data as *const T) };
      data.inline_view(ui)
    });
    self.hover_inspect_ui.insert(TypeId::of::<T>(), |data, ui| {
      let data = unsafe { &*(data as *const T) };
      data.hover_detail_view(ui);
    });
    self
  }

  pub fn ui(&self, tid: &TypeId, data: *const (), ui: &mut egui::Ui, fallback_debug: String) {
    if let Some(f) = self.inline_ui.get(tid) {
      let res = f(data, ui);
      if let Some(f) = self.hover_inspect_ui.get(tid) {
        res.on_hover_ui(|ui| {
          f(data, ui);
        });
      }
    } else {
      fn truncate(s: &str, max_chars: usize) -> &str {
        match s.char_indices().nth(max_chars) {
          None => s,
          Some((idx, _)) => &s[..idx],
        }
      }

      ui.weak(truncate(&fallback_debug, 24))
        .on_hover_ui(move |ui| {
          ui.label(fallback_debug);
        });
    }
  }
}

pub trait EGUIDataView: std::fmt::Debug {
  // provide brief information
  fn inline_view(&self, ui: &mut egui::Ui) -> egui::Response;
  // provide more detailed information, the default impl will be the debug string
  fn hover_detail_view(&self, ui: &mut egui::Ui) {
    ui.label(format!("{:?}", self));
  }
}

impl EGUIDataView for f32 {
  fn inline_view(&self, ui: &mut egui::Ui) -> egui::Response {
    ui.label(format!("{:.2}", self))
  }

  fn hover_detail_view(&self, ui: &mut egui::Ui) {
    ui.label(format!("{:?}", self));
  }
}

#[allow(clippy::format_collect)]
fn display_float_array(array: &[f32]) -> String {
  array
    .iter()
    .map(|v| format!("{:.2}, ", v))
    .collect::<String>()
}

impl EGUIDataView for Option<RawEntityHandle> {
  fn inline_view(&self, ui: &mut egui::Ui) -> egui::Response {
    ui.label(format!("{:?}", self.map(|v| v.index())))
  }
}

impl EGUIDataView for Vec2<f32> {
  fn inline_view(&self, ui: &mut egui::Ui) -> egui::Response {
    let array = bytes_of(self);
    let array = cast_slice::<u8, f32>(array);
    ui.label(display_float_array(array))
  }
}
impl EGUIDataView for Vec3<f32> {
  fn inline_view(&self, ui: &mut egui::Ui) -> egui::Response {
    let array = bytes_of(self);
    let array = cast_slice::<u8, f32>(array);
    ui.label(display_float_array(array))
  }
}
impl EGUIDataView for Vec4<f32> {
  fn inline_view(&self, ui: &mut egui::Ui) -> egui::Response {
    let array = bytes_of(self);
    let array = cast_slice::<u8, f32>(array);
    ui.label(display_float_array(array))
  }
}

impl EGUIDataView for Mat4<f64> {
  fn inline_view(&self, ui: &mut egui::Ui) -> egui::Response {
    let array = bytes_of(self);
    let array = cast_slice::<u8, f32>(array);

    ui.label(display_float_array(array))
  }
  fn hover_detail_view(&self, ui: &mut egui::Ui) {
    ui.label(format!(
      "[{}, {}, {}, {}]",
      self.a1, self.a2, self.a3, self.a4,
    ));
    ui.label(format!(
      "[{}, {}, {}, {}]",
      self.b1, self.b2, self.b3, self.b4,
    ));
    ui.label(format!(
      "[{}, {}, {}, {}]",
      self.c1, self.c2, self.c3, self.c4,
    ));
    ui.label(format!(
      "[{}, {}, {}, {}]",
      self.d1, self.d2, self.d3, self.d4,
    ));
  }
}

fn all_tables(ui: &mut egui::Ui, state: &mut DBInspector) {
  ui.heading("Tables");

  let db = global_database();
  let db_tables = db.tables.read_recursive();

  let table = TableBuilder::new(ui)
    .striped(true)
    .column(Column::auto())
    .column(Column::auto())
    .column(Column::auto())
    .column(Column::auto())
    .cell_layout(egui::Layout::left_to_right(egui::Align::Center));

  let mut all_memory_in_bytes = 0;

  table
    .header(20.0, |mut header| {
      header.col(|ui| {
        ui.strong("name");
      });
      header.col(|ui| {
        ui.strong("count");
      });
      header.col(|ui| {
        ui.strong("capacity");
      });
      header.col(|ui| {
        ui.strong("memory_usage")
          .on_hover_text("not include indirect component heap data");
      });
    })
    .body(|mut body| {
      for (id, db_table) in db_tables.iter() {
        body.row(20.0, |mut row| {
          row.col(|ui| {
            if ui.link(db_table.short_name()).clicked() {
              state.goto(Some(*id));
            }
          });
          row.col(|ui| {
            ui.label(db_table.living_entity_count().to_string());
          });
          row.col(|ui| {
            ui.label(db_table.entity_capacity().to_string());
          });
          row.col(|ui| {
            let bytes = db_table.memory_usage_in_bytes();
            all_memory_in_bytes += bytes;
            let size = humansize::format_size(bytes, humansize::BINARY);
            ui.label(size);
          });
        })
      }
    });

  let size = humansize::format_size(all_memory_in_bytes, humansize::BINARY);
  ui.label(format!("all data in db memory usage: {}", size));
}
