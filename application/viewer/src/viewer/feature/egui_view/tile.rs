use egui_tiles::*;
use fast_hash_collection::FastHashSet;

use crate::*;

pub fn use_egui_tile_for_viewer_viewports(cx: &mut ViewerCx) {
  let (cx, tree) =
    cx.use_plain_state_init(|init| create_viewer_default_tile_tree(&init.content.viewports));

  let all_scene_cameras = cx
    .use_db_rev_ref::<SceneCameraBelongsToScene>()
    .use_assure_result(cx);
  let (cx, all_scene_cameras_cached) = cx.use_plain_state();

  if cx.is_resolve_stage() {
    *all_scene_cameras_cached = all_scene_cameras
      .expect_resolve_stage()
      .access_multi(&cx.viewer.content.scene.into_raw())
      .unwrap()
      .collect::<FastHashSet<RawEntityHandle>>();
  }

  if let ViewerCxStage::Gui { egui_ctx, .. } = &mut cx.stage {
    let mut behavior = ViewerTileTreeBehavior {
      camera_handles: all_scene_cameras_cached.clone(),
      ..Default::default()
    };

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
      cx.dyn_cx.message.put(CameraControlBlocked);
      cx.dyn_cx.message.put(PickSceneBlocked);
    }

    if let Some(tile_id) = behavior.remove_tile.take() {
      for tile in tree.remove_recursively(tile_id) {
        if let egui_tiles::Tile::Pane(pane) = tile {
          let removed_viewport_id = pane.viewport_id;
          let idx = cx
            .viewer
            .content
            .viewports
            .iter()
            .position(|v| v.id == removed_viewport_id)
            .unwrap();
          cx.viewer.content.viewports.remove(idx);
        }
      }
    }

    if let Some(_request_tile) = behavior.add_child_to.take() {
      let camera_source = cx.viewer.content.viewports.last().unwrap();
      let id = alloc_global_res_id();
      let new_viewport = ViewerViewPort {
        id,
        viewport: Default::default(),
        camera: camera_source.camera,
        camera_node: camera_source.camera_node,
        debug_camera_for_view_related: None,
      };
      cx.viewer.content.viewports.push(new_viewport);

      let any_camera_in_target_scene = *all_scene_cameras_cached.iter().next().unwrap();
      let new_child = tree
        .tiles
        .insert_pane(ViewerPane::new(id, any_camera_in_target_scene));
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
    let camera_nodes = get_db_view_typed_foreign::<SceneCameraNode>();
    let mut camera_perspective_proj =
      global_entity_component_of::<SceneCameraPerspective>().write();
    let mut camera_orth_proj = global_entity_component_of::<SceneCameraOrthographic>().write();

    for tile_id in tree.active_tiles() {
      if let Some(tile) = tree.tiles.get_mut(tile_id) {
        if let egui_tiles::Tile::Pane(pane) = tile {
          if let Some(viewport) = cx
            .viewer
            .content
            .viewports
            .iter_mut()
            .find(|viewport| viewport.id == pane.viewport_id)
          {
            let r = pane.rect;
            let ratio = cx.input.window_state.device_pixel_ratio;
            let width = r.width() * ratio;
            let height = r.height() * ratio;
            viewport.viewport = (r.min.x * ratio, r.min.y * ratio, width, height).into();
            let camera = unsafe { EntityHandle::from_raw(pane.camera_handle) };
            viewport.camera = camera;
            viewport.camera_node = camera_nodes.access(&camera).unwrap();
            viewport.debug_camera_for_view_related = pane
              .debug_view_camera_handle
              .map(|h| unsafe { EntityHandle::from_raw(h) });

            if pane.request_switch_proj {
              pane.request_switch_proj = false;
              println!("request switch proj");

              if let Some(_perspective) = camera_perspective_proj.read(viewport.camera).flatten() {
                camera_perspective_proj.write(viewport.camera, None);
                camera_orth_proj.write(viewport.camera, Some(OrthographicProjection::default()));
              } else if let Some(_orth) = camera_orth_proj.read(viewport.camera).flatten() {
                camera_orth_proj.write(viewport.camera, None);
                camera_perspective_proj
                  .write(viewport.camera, Some(PerspectiveProjection::default()));
              }
            }
          } // or else tile get removed(viewport get removed)
        }
      } // or else new get removed
    }
  }
}

#[derive(Debug)]
pub struct ViewerPane {
  pub viewport_id: u64,
  pub rect: egui::Rect,
  pub show_camera_setting: bool,
  pub camera_handle: RawEntityHandle,
  pub debug_view_camera_handle: Option<RawEntityHandle>,
  pub request_switch_proj: bool,
}

impl ViewerPane {
  pub fn new(viewport_id: u64, camera_handle: RawEntityHandle) -> Self {
    ViewerPane {
      viewport_id,
      show_camera_setting: false,
      rect: egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(0., 0.)),
      camera_handle,
      debug_view_camera_handle: None,
      request_switch_proj: false,
    }
  }
}

#[derive(Default)]
pub struct ViewerTileTreeBehavior {
  pub camera_handles: FastHashSet<RawEntityHandle>,
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

          if ui.button("camera").clicked() {
            pane.show_camera_setting = !pane.show_camera_setting;
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

    if pane.show_camera_setting {
      ui.horizontal(|ui| {
        egui::frame::Frame::NONE
          .inner_margin(egui::Margin::same(3))
          .show(ui, |ui| {
            if ui.button("switch_proj").clicked() {
              pane.request_switch_proj = true;
            }

            egui::ComboBox::from_label("camera")
              .selected_text(format!("{:?}", pane.camera_handle))
              .show_ui(ui, |ui| {
                for c in self.camera_handles.iter() {
                  ui.selectable_value(&mut pane.camera_handle, *c, format!("{:?}", c));
                }
              });

            egui::ComboBox::from_label("debug view camera")
              .selected_text(format!("{:?}", pane.debug_view_camera_handle))
              .show_ui(ui, |ui| {
                ui.selectable_value(&mut pane.debug_view_camera_handle, None, "none");
                for c in self.camera_handles.iter() {
                  ui.selectable_value(
                    &mut pane.debug_view_camera_handle,
                    Some(*c),
                    format!("{:?}", c),
                  );
                }
              });
          });
      });
    }

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
      let pane = ViewerPane::new(viewport.id, viewport.camera.into_raw());
      tiles.insert_pane(pane)
    })
    .collect();
  let root = tiles.insert_horizontal_tile(children);

  egui_tiles::Tree::new("viewer tree", root, tiles)
}
