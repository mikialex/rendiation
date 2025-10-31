use egui_tiles::*;

use crate::*;

pub struct ViewerPane {
  nr: usize,
}

pub struct ViewerTileTreeBehavior {
  pub edited: std::cell::Cell<bool>,
}

impl egui_tiles::Behavior<ViewerPane> for ViewerTileTreeBehavior {
  fn tab_title_for_pane(&mut self, pane: &ViewerPane) -> egui::WidgetText {
    format!("Pane {}", pane.nr).into()
  }

  fn on_edit(&mut self, _edit_action: egui_tiles::EditAction) {
    self.edited.set(true);
  }

  fn resize_stroke(&self, style: &egui::Style, resize_state: ResizeState) -> egui::Stroke {
    match resize_state {
      ResizeState::Idle => {
        egui::Stroke::new(self.gap_width(style), self.tab_bar_color(&style.visuals))
      }
      ResizeState::Hovering => {
        self.edited.set(true); // this is a hack
        style.visuals.widgets.hovered.fg_stroke
      }
      ResizeState::Dragging => style.visuals.widgets.active.fg_stroke,
    }
  }

  fn pane_ui(
    &mut self,
    ui: &mut egui::Ui,
    _tile_id: egui_tiles::TileId,
    _pane: &mut ViewerPane,
  ) -> egui_tiles::UiResponse {
    // // Give each pane a unique color:
    // let color = egui::epaint::Hsva::new(0.103 * pane.nr as f32, 0.5, 0.5, 0.5);
    // ui.painter().rect_filled(ui.max_rect(), 0.0, color);

    // You can make your pane draggable like so:
    if ui
      .add(egui::Button::new("Drag").sense(egui::Sense::drag()))
      .drag_started()
    {
      self.edited.set(true);
      egui_tiles::UiResponse::DragStarted
    } else {
      egui_tiles::UiResponse::None
    }
  }
}

pub fn create_viewer_default_tile_tree() -> egui_tiles::Tree<ViewerPane> {
  let mut next_view_nr = 0;
  let mut gen_pane = || {
    let pane = ViewerPane { nr: next_view_nr };
    next_view_nr += 1;
    pane
  };

  let mut tiles = egui_tiles::Tiles::default();

  let mut tabs = vec![];
  tabs.push({
    let children = (0..2).map(|_| tiles.insert_pane(gen_pane())).collect();
    tiles.insert_horizontal_tile(children)
  });
  //   tabs.push({
  //     let cells = (0..11).map(|_| tiles.insert_pane(gen_pane())).collect();
  //     tiles.insert_grid_tile(cells)
  //   });
  //   tabs.push(tiles.insert_pane(gen_pane()));

  let root = tiles.insert_tab_tile(tabs);

  egui_tiles::Tree::new("viewer tree", root, tiles)
}
