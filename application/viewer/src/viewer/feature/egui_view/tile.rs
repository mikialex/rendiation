use egui_tiles::*;

use crate::*;

pub fn use_egui_tile_for_viewer_viewports(
  acx: &mut ApplicationCx,
  egui_ctx: &mut egui::Context,
  viewer: &mut Viewer,
) {
  let (acx, tree) =
    acx.use_plain_state(|| create_viewer_default_tile_tree(&viewer.content.viewports));

  let mut behavior = ViewerTileTreeBehavior::default();

  let tree_res = egui::CentralPanel::default()
    .frame(egui::Frame::NONE)
    .show(egui_ctx, |ui| {
      tree.ui(&mut behavior, ui);
    });

  /// Is the pointer (mouse/touch) over any egui area?
  fn is_pointer_over_area_no_view_tree(cx: &egui::Context, tree_layer_id: egui::LayerId) -> bool {
    let pointer_pos = cx.input(|i| i.pointer.interact_pos());
    if let Some(pointer_pos) = pointer_pos {
      if let Some(layer) = cx.layer_id_at(pointer_pos) {
        tree_layer_id != layer
      } else {
        false
      }
    } else {
      false
    }
  }

  let tree_layer_id = tree_res.response.layer_id;
  if is_pointer_over_area_no_view_tree(egui_ctx, tree_layer_id) || behavior.edited.get() {
    acx.dyn_cx.message.put(CameraControlBlocked);
    acx.dyn_cx.message.put(PickSceneBlocked);
  }

  if let Some(tile_id) = behavior.remove_tile.take() {
    for tile in tree.remove_recursively(tile_id) {
      if let egui_tiles::Tile::Pane(pane) = tile {
        let removed_viewport_id = pane.viewport_id;
        let idx = viewer
          .content
          .viewports
          .iter()
          .position(|v| v.id == removed_viewport_id)
          .unwrap();
        viewer.content.viewports.remove(idx);
      }
    }
  }

  if let Some(_request_tile) = behavior.add_child_to.take() {
    let camera_source = viewer.content.viewports.last().unwrap();
    let id = alloc_global_res_id();
    let new_viewport = ViewerViewPort {
      id,
      viewport: Default::default(),
      camera: camera_source.camera,
      camera_node: camera_source.camera_node,
    };
    viewer.content.viewports.push(new_viewport);

    let new_child = tree.tiles.insert_pane(ViewerPane::new(id));
    if let Some(root) = tree.root() {
      if let egui_tiles::Tile::Container(egui_tiles::Container::Linear(container)) =
        tree.tiles.get_mut(root).unwrap()
      {
        container.add_child(new_child);
      } else {
        log::error!("unable to add child to none container root, this is a bug")
      }
    }
  }

  // sync the viewer viewports
  for tile_id in tree.active_tiles() {
    if let Some(tile) = tree.tiles.get(tile_id) {
      if let egui_tiles::Tile::Pane(pane) = tile {
        if let Some(viewport) = viewer
          .content
          .viewports
          .iter_mut()
          .find(|viewport| viewport.id == pane.viewport_id)
        {
          let r = pane.rect;
          let ratio = acx.input.window_state.device_pixel_ratio;
          let width = r.width() * ratio;
          let height = r.height() * ratio;
          viewport.viewport = (r.min.x * ratio, r.min.y * ratio, width, height).into();
        } // or else tile get removed(viewport get removed)
      }
    } // or else new get removed
  }
}

#[derive(Debug)]
pub struct ViewerPane {
  pub viewport_id: u64,
  pub rect: egui::Rect,
}

impl ViewerPane {
  pub fn new(viewport_id: u64) -> Self {
    ViewerPane {
      viewport_id,
      rect: egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(0., 0.)),
    }
  }
}

#[derive(Default)]
pub struct ViewerTileTreeBehavior {
  pub edited: std::cell::Cell<bool>,
  pub add_child_to: Option<TileId>,
  pub remove_tile: Option<TileId>,
}

impl egui_tiles::Behavior<ViewerPane> for ViewerTileTreeBehavior {
  fn simplification_options(&self) -> SimplificationOptions {
    SimplificationOptions {
      prune_empty_tabs: true,
      prune_empty_containers: false,
      prune_single_child_tabs: true,
      prune_single_child_containers: false,
      all_panes_must_have_tabs: false,
      join_nested_linear_containers: false,
    }
  }

  fn tab_title_for_pane(&mut self, pane: &ViewerPane) -> egui::WidgetText {
    format!("viewport {}", pane.viewport_id).into()
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

  fn top_bar_right_ui(
    &mut self,
    _tiles: &egui_tiles::Tiles<ViewerPane>,
    ui: &mut egui::Ui,
    tile_id: egui_tiles::TileId,
    _tabs: &egui_tiles::Tabs,
    _scroll_offset: &mut f32,
  ) {
    if ui.button("+").clicked() {
      self.add_child_to = Some(tile_id);
    }
  }

  fn is_tab_closable(&self, _tiles: &Tiles<ViewerPane>, _tile_id: TileId) -> bool {
    true
  }

  fn on_tab_close(&mut self, _tiles: &mut Tiles<ViewerPane>, tile_id: TileId) -> bool {
    self.remove_tile = Some(tile_id);

    true
  }

  fn pane_ui(
    &mut self,
    ui: &mut egui::Ui,
    tile_id: egui_tiles::TileId,
    pane: &mut ViewerPane,
  ) -> egui_tiles::UiResponse {
    pane.rect = ui.max_rect();

    let mut r = egui_tiles::UiResponse::None;
    ui.spacing_mut().window_margin = egui::Margin::same(3);

    ui.horizontal(|ui| {
      egui::frame::Frame::NONE
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
          if ui.button("+").clicked() {
            self.add_child_to = Some(tile_id);
          }

          if ui.button("x").clicked() {
            self.remove_tile = Some(tile_id);
          }

          if ui
            .add(egui::Button::new("Drag").sense(egui::Sense::drag()))
            .drag_started()
          {
            self.edited.set(true);
            r = egui_tiles::UiResponse::DragStarted;
          }
        })
    });

    r
  }
}

pub fn create_viewer_default_tile_tree(
  viewports: &[ViewerViewPort],
) -> egui_tiles::Tree<ViewerPane> {
  let mut tiles = egui_tiles::Tiles::default();

  let children = viewports
    .iter()
    .map(|viewport| {
      let pane = ViewerPane::new(viewport.id);
      tiles.insert_pane(pane)
    })
    .collect();
  let root = tiles.insert_horizontal_tile(children);

  egui_tiles::Tree::new("viewer tree", root, tiles)
}
