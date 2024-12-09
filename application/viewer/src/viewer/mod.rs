use crate::*;

mod feature;
mod pick;
use default_scene::load_default_scene;
pub use feature::*;

mod terminal;
pub use pick::*;
pub use terminal::*;

mod egui_console;
pub use egui_console::*;

mod rendering;
pub use rendering::*;

pub struct Viewer {
  widget_intersection_group: WidgetSceneModelIntersectionGroupConfig,
  on_demand_rendering: bool,
  on_demand_draw: NotifyScope,
  scene: Viewer3dSceneCtx,
  rendering: Viewer3dRenderingCtx,
  derives: Viewer3dSceneDeriveSource,
  content: Box<dyn Widget>,
  ui_state: ViewerUIState,
  egui_db_inspector: db_egui_view::DBInspector,
  terminal: Terminal,
}

struct ViewerUIState {
  show_db_inspector: bool,
  show_viewer_config_panel: bool,
  show_terminal: bool,
}

impl Widget for Viewer {
  fn update_state(&mut self, cx: &mut DynCx) {
    let mut derived = self.derives.poll_update();

    cx.scoped_cx(&mut derived, |cx| {
      cx.scoped_cx(&mut self.scene, |cx| {
        access_cx!(cx, input, PlatformEventInput);
        access_cx!(cx, derived, Viewer3dSceneDerive);
        access_cx!(cx, viewer_scene, Viewer3dSceneCtx);

        let main_camera_handle = viewer_scene.main_camera;

        self.rendering.update_next_render_camera_info(
          derived
            .camera_transforms
            .access(&main_camera_handle)
            .unwrap()
            .view_projection_inv,
        );

        let picker = ViewerPicker::new(derived, input, main_camera_handle);

        let mut scene_reader = SceneReader::new_from_global(
          viewer_scene.scene,
          derived.mesh_vertex_ref.clone(),
          derived.node_children.clone(),
          derived.sm_to_s.clone(),
        );
        let mut widget_derive_access = Box::new(WidgetEnvAccessImpl {
          world_mat: derived.world_mat.clone(),
          camera_node: viewer_scene.camera_node,
          camera_proj: scene_reader
            .camera
            .read::<SceneCameraPerspective>(viewer_scene.main_camera)
            .unwrap(),
          canvas_resolution: Vec2::new(
            input.window_state.size.0 as u32, //todo, fix , this should use render resolution instead of window size
            input.window_state.size.1 as u32,
          ),
          camera_world_ray: picker.current_mouse_ray_in_world(),
          normalized_canvas_position: picker.normalized_position(),
        }) as Box<dyn WidgetEnvAccess>;

        let mut interaction_cx = prepare_picking_state(picker, &self.widget_intersection_group);

        cx.scoped_cx(&mut widget_derive_access, |cx| {
          cx.scoped_cx(&mut scene_reader, |cx| {
            cx.scoped_cx(&mut self.widget_intersection_group, |cx| {
              cx.scoped_cx(&mut interaction_cx, |cx| {
                cx.scoped_cx(&mut self.rendering, |cx| {
                  self.content.update_state(cx);
                });
              });
            });
          });
        });
      });
    });
  }
  fn update_view(&mut self, cx: &mut DynCx) {
    access_cx!(cx, platform, PlatformEventInput);
    let size = platform.window_state.size;
    let size_changed = platform.state_delta.size_change;
    if size_changed {
      self.rendering.resize_view()
    }

    cx.scoped_cx(&mut self.scene, |cx| {
      cx.scoped_cx(&mut self.derives, |cx| {
        cx.split_cx::<egui::Context>(|egui_cx, cx| {
          egui(
            &mut self.terminal,
            &mut self.on_demand_rendering,
            &mut self.rendering,
            egui_cx,
            &mut self.ui_state,
            cx,
          );
          if self.ui_state.show_db_inspector {
            crate::db_egui_view::egui_db_gui(egui_cx, &mut self.egui_db_inspector);
          }
        });
      });

      access_cx!(cx, viewer_scene, Viewer3dSceneCtx);
      let mut writer = SceneWriter::from_global(viewer_scene.scene);

      if size_changed {
        writer
          .camera_writer
          .mutate_component_data::<SceneCameraPerspective>(viewer_scene.main_camera, |p| {
            if let Some(p) = p.as_mut() {
              p.resize(size)
            }
          });
      }

      cx.scoped_cx(&mut writer, |cx| {
        cx.scoped_cx(&mut self.rendering, |cx| {
          self.content.update_view(cx);
        });
      });
    });

    access_cx!(cx, draw_target_canvas, RenderTargetView);
    self.draw_canvas(draw_target_canvas);
  }

  fn clean_up(&mut self, cx: &mut DynCx) {
    let mut writer = SceneWriter::from_global(self.scene.scene);
    cx.scoped_cx(&mut writer, |cx| self.content.clean_up(cx));
  }
}

impl Viewer {
  pub fn new(gpu: GPU, content_logic: impl Widget + 'static) -> Self {
    let mut terminal = Terminal::default();
    register_default_commands(&mut terminal);

    let scene = global_entity_of::<SceneEntity>()
      .entity_writer()
      .new_entity();

    let widget_scene = global_entity_of::<SceneEntity>()
      .entity_writer()
      .new_entity();

    let root = global_entity_of::<SceneNodeEntity>()
      .entity_writer()
      .new_entity();

    let camera_node = global_entity_of::<SceneNodeEntity>()
      .entity_writer()
      .with_component_value_writer::<SceneNodeLocalMatrixComponent>(Mat4::lookat(
        Vec3::new(3., 3., 3.),
        Vec3::new(0., 0., 0.),
        Vec3::new(0., 1., 0.),
      ))
      .new_entity();

    let main_camera = global_entity_of::<SceneCameraEntity>()
      .entity_writer()
      .with_component_value_writer::<SceneCameraPerspective>(Some(PerspectiveProjection::default()))
      .with_component_value_writer::<SceneCameraBelongsToScene>(Some(scene.into_raw()))
      .with_component_value_writer::<SceneCameraNode>(Some(camera_node.into_raw()))
      .new_entity();

    let scene = Viewer3dSceneCtx {
      main_camera,
      camera_node,
      scene,
      root,
      selected_target: None,
      widget_scene,
    };

    {
      let mut writer = SceneWriter::from_global(scene.scene);
      load_default_scene(&mut writer, &scene);
    }

    let derives = Viewer3dSceneDeriveSource {
      world_mat: Box::new(scene_node_derive_world_mat()),
      node_net_visible: Box::new(scene_node_derive_visible()),
      camera_transforms: Box::new(camera_transforms()),
      mesh_vertex_ref: Box::new(
        global_rev_ref()
          .watch_inv_ref::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>(),
      ),
      sm_to_s: Box::new(global_rev_ref().watch_inv_ref::<SceneModelBelongsToScene>()),
      mesh_local_bounding: Box::new(attribute_mesh_local_bounding()),
      node_children: Box::new(scene_node_connectivity_many_one_relation()),
    };

    Self {
      widget_intersection_group: Default::default(),
      // todo, we current disable the on demand draw
      // because we not cache the rendering result yet
      on_demand_rendering: false,
      ui_state: ViewerUIState {
        show_db_inspector: false,
        show_viewer_config_panel: true,
        show_terminal: true,
      },
      content: Box::new(content_logic),
      scene,
      terminal,
      rendering: Viewer3dRenderingCtx::new(gpu),
      derives,
      on_demand_draw: Default::default(),
      egui_db_inspector: Default::default(),
    }
  }

  pub fn draw_canvas(&mut self, canvas: &RenderTargetView) {
    if !self.on_demand_rendering {
      self.on_demand_draw.wake();
    }

    self.on_demand_draw.update_once(|cx| {
      // println!("draw");
      self.rendering.render(canvas.clone(), &self.scene, cx)
    });
  }
}

fn egui(
  terminal: &mut Terminal,
  on_demand_rendering: &mut bool,
  rendering: &mut Viewer3dRenderingCtx,
  ui: &egui::Context,
  ui_state: &mut ViewerUIState,
  cx: &mut DynCx,
) {
  egui::TopBottomPanel::top("view top menu").show(ui, |ui| {
    ui.horizontal_wrapped(|ui| {
      egui::widgets::global_dark_light_mode_switch(ui);
      ui.separator();
      ui.checkbox(&mut ui_state.show_db_inspector, "database inspector");
      ui.checkbox(&mut ui_state.show_viewer_config_panel, "viewer config");
      ui.checkbox(&mut ui_state.show_terminal, "terminal");
    });
  });

  if ui_state.show_viewer_config_panel {
    egui::Window::new("Viewer")
      .vscroll(true)
      .default_open(true)
      .default_pos([10., 60.])
      .max_width(1000.0)
      .max_height(800.0)
      .default_width(250.0)
      .default_height(300.0)
      .resizable(true)
      .movable(true)
      .show(ui, |ui| {
        if ui.button("Organize windows").clicked() {
          ui.ctx().memory_mut(|mem| mem.reset_areas());
        }

        ui.separator();
        ui.checkbox(on_demand_rendering, "enable on demand rendering");
        ui.separator();
        rendering.egui(ui);
        ui.separator();

        ui.collapsing("Instance Counts", |ui| {
          let mut counters = heap_tools::HEAP_TOOL_GLOBAL_INSTANCE_COUNTER
            .write()
            .unwrap();

          for (name, r) in counters.report_all_instance_count() {
            ui.label(format!(
              "{}: current:{} peak:{}",
              get_short_name(name),
              r.current,
              r.history_peak
            ));
          }

          if ui.button("reset peak").clicked() {
            counters.reset_all_instance_history_peak();
          }
        });
      });
  }

  if ui_state.show_terminal {
    egui::TopBottomPanel::bottom("view bottom terminal").show(ui, |ui| {
      cx.scoped_cx(rendering, |cx| {
        terminal.egui(ui, cx);
      });
    });
  }
}

pub struct Viewer3dSceneCtx {
  pub main_camera: EntityHandle<SceneCameraEntity>,
  pub camera_node: EntityHandle<SceneNodeEntity>,
  pub root: EntityHandle<SceneNodeEntity>,
  pub scene: EntityHandle<SceneEntity>,
  pub selected_target: Option<EntityHandle<SceneModelEntity>>,
  pub widget_scene: EntityHandle<SceneEntity>,
}

pub struct Viewer3dSceneDeriveSource {
  pub world_mat: BoxedDynReactiveQuery<EntityHandle<SceneNodeEntity>, Mat4<f32>>,
  pub node_net_visible: BoxedDynReactiveQuery<EntityHandle<SceneNodeEntity>, bool>,
  pub camera_transforms: BoxedDynReactiveQuery<EntityHandle<SceneCameraEntity>, CameraTransform>,
  pub mesh_vertex_ref:
    RevRefOfForeignKeyWatch<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>,
  pub sm_to_s: RevRefOfForeignKeyWatch<SceneModelBelongsToScene>,
  pub mesh_local_bounding: BoxedDynReactiveQuery<EntityHandle<AttributesMeshEntity>, Box3<f32>>,
  pub node_children:
    BoxedDynReactiveOneToManyRelation<EntityHandle<SceneNodeEntity>, EntityHandle<SceneNodeEntity>>,
}

impl Viewer3dSceneDeriveSource {
  fn poll_update(&self) -> Viewer3dSceneDerive {
    let waker = futures::task::noop_waker_ref();
    let mut cx = Context::from_waker(waker);
    let cx = &mut cx;

    let (_, world_mat) = self.world_mat.poll_changes(cx);
    let (_, node_net_visible) = self.node_net_visible.poll_changes(cx);
    let (_, camera_transforms) = self.camera_transforms.poll_changes(cx);
    let (_, _, mesh_vertex_ref) = self.mesh_vertex_ref.poll_changes_with_inv_dyn(cx);
    let (_, _, sm_to_s) = self.sm_to_s.poll_changes_with_inv_dyn(cx);
    let (_, mesh_local_bounding) = self.mesh_local_bounding.poll_changes(cx);
    let (_, _, node_children) = self.node_children.poll_changes_with_inv_dyn(cx);
    Viewer3dSceneDerive {
      world_mat,
      camera_transforms,
      mesh_vertex_ref,
      node_net_visible,
      mesh_local_bounding,
      node_children,
      sm_to_s,
    }
  }
}

/// used in render & scene update
#[derive(Clone)]
pub struct Viewer3dSceneDerive {
  pub world_mat: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f32>>,
  pub node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  pub node_children:
    BoxedDynMultiQuery<EntityHandle<SceneNodeEntity>, EntityHandle<SceneNodeEntity>>,
  pub camera_transforms: BoxedDynQuery<EntityHandle<SceneCameraEntity>, CameraTransform>,
  pub mesh_vertex_ref:
    RevRefOfForeignKey<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>,
  pub mesh_local_bounding: BoxedDynQuery<EntityHandle<AttributesMeshEntity>, Box3<f32>>,
  pub sm_to_s: RevRefOfForeignKey<SceneModelBelongsToScene>,
}

struct WidgetEnvAccessImpl {
  world_mat: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f32>>,
  camera_node: EntityHandle<SceneNodeEntity>,
  camera_proj: PerspectiveProjection<f32>,
  canvas_resolution: Vec2<u32>,
  camera_world_ray: Ray3,
  // xy -1 to 1
  normalized_canvas_position: Vec2<f32>,
}

impl WidgetEnvAccess for WidgetEnvAccessImpl {
  fn get_world_mat(&self, sm: EntityHandle<SceneNodeEntity>) -> Option<Mat4<f32>> {
    self.world_mat.access(&sm)
  }

  fn get_camera_node(&self) -> EntityHandle<SceneNodeEntity> {
    self.camera_node
  }

  fn get_camera_perspective_proj(&self) -> PerspectiveProjection<f32> {
    self.camera_proj
  }

  fn get_camera_world_ray(&self) -> Ray3 {
    self.camera_world_ray
  }

  fn get_normalized_canvas_position(&self) -> Vec2<f32> {
    self.normalized_canvas_position
  }

  fn get_view_resolution(&self) -> Vec2<u32> {
    self.canvas_resolution
  }
}
