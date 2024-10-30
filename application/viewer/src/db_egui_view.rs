use egui_extras::{Column, TableBuilder};

use crate::*;

pub struct DBInspector {
  visit_history: Vec<Option<EntityId>>,
  current: usize,
}

impl Default for DBInspector {
  fn default() -> Self {
    Self {
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

pub fn egui_db_gui(ui: &mut egui::Context, state: &mut DBInspector) {
  egui::Window::new("Database Inspector")
    .default_open(false)
    .min_width(500.0)
    .max_width(700.0)
    .min_height(400.0)
    .max_height(1000.0)
    .default_width(800.0)
    .default_height(400.)
    .resizable(true)
    .movable(true)
    .default_pos([10., 10.])
    .scroll([true, true])
    .show(ui, |ui| {
      let mut back_to_all_table_view = false;
      ui.horizontal_wrapped(|ui| {
        // ui.with_layout(Layout::left_to_right().with_main_justify( true), |ui|{

        // });
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
  db.access_ecg_dyn(e_id, |ecg| {
    ui.heading(ecg.name());

    let table = TableBuilder::new(ui)
      .striped(true)
      .column(Column::auto())
      .columns(
        Column::auto().resizable(true).at_most(500.).clip(true),
        ecg.component_count(),
      )
      .max_scroll_height(900.)
      .cell_layout(egui::Layout::left_to_right(egui::Align::Center));

    ecg.access_components(|coms| {
      table
        .header(20.0, |mut header| {
          header.col(|ui| {
            ui.strong("entity id");
          });
          coms.values().for_each(|com| {
            header.col(|ui| {
              ui.strong(com.name.clone());

              if let Some(f) = com.as_foreign_key {
                let name = db.access_ecg_dyn(f, |ecg| ecg.name().to_string());
                if ui.link(name).clicked() {
                  state.goto(Some(f));
                }
              }
            });
          })
        })
        .body(|body| {
          body.rows(20.0, ecg.entity_allocation_count(), |mut row| {
            let idx = row.index();
            row.col(|ui| {
              ui.label(idx.to_string());
            });
            coms.values().for_each(|com| {
              row.col(|ui| {
                if let Some(value) = com.debug_value(idx) {
                  ui.label(value);
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

fn all_tables(ui: &mut egui::Ui, state: &mut DBInspector) {
  ui.heading("Tables");

  let db = global_database();
  let db_tables = db.ecg_tables.read_recursive();

  let table = TableBuilder::new(ui)
    .striped(true)
    .column(Column::auto())
    .column(Column::auto())
    .column(Column::auto())
    .cell_layout(egui::Layout::left_to_right(egui::Align::Center));

  table
    .header(20.0, |mut header| {
      header.col(|ui| {
        ui.strong("Entity Name");
      });
      header.col(|ui| {
        ui.strong("Entity Count");
      });
    })
    .body(|mut body| {
      for (id, db_table) in db_tables.iter() {
        body.row(20.0, |mut row| {
          row.col(|ui| {
            if ui.link(db_table.name()).clicked() {
              state.goto(Some(*id));
            }
          });
          row.col(|ui| {
            ui.label(db_table.entity_count().to_string());
          });
        })
      }
    });
}
