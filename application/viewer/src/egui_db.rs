use egui_extras::{Column, TableBuilder};

use crate::*;

#[derive(Default)]
pub struct DBInspector {
  visiting_entity: Option<EntityId>,
}

pub fn egui_db_gui(ui: &mut egui::Context, state: &mut DBInspector) {
  egui::Window::new("Database Inspector")
    .vscroll(true)
    .default_open(true)
    .min_width(500.0)
    .max_width(1000.0)
    .min_height(400.0)
    .max_height(1000.0)
    .default_width(800.0)
    .resizable(true)
    .movable(true)
    .scroll2([true, true])
    .show(ui, |ui| {
      if let Some(visiting_entity) = &state.visiting_entity {
        let back_to_all_table_view = ui.button("View all tables in DB").clicked();
        ui.separator();

        selected_table(ui, state, *visiting_entity);

        if back_to_all_table_view {
          state.visiting_entity = None;
        }
      } else {
        all_tables(ui, state);
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
        Column::auto().at_most(200.).clip(true).resizable(true),
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
        })
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
              state.visiting_entity = Some(*id);
            }
          });
          row.col(|ui| {
            ui.label(db_table.entity_count().to_string());
          });
        })
      }
    });
}
