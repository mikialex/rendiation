use std::time::Instant;

use crate::*;

mod feature;
pub use feature::*;

mod default_scene;
pub use default_scene::*;

mod pick;
pub use pick::*;

mod terminal;
pub use terminal::*;

mod animation_player;
pub use animation_player::*;

mod background;
pub use background::*;

mod test_content;
pub use test_content::*;

mod console;
pub use console::*;

mod rendering;
pub use rendering::*;

pub const UP: Vec3<f32> = Vec3::new(0., 1., 0.);

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
  background: ViewerBackgroundState,
  camera_helpers: SceneCameraHelper,
  animation_player: SceneAnimationsPlayer,
  started_time: Instant,
}

struct ViewerUIState {
  show_db_inspector: bool,
  show_viewer_config_panel: bool,
  show_terminal: bool,
  show_gpu_info: bool,
  show_memory_stat: bool,
}

impl Default for ViewerUIState {
  fn default() -> Self {
    Self {
      show_db_inspector: false,
      show_viewer_config_panel: true,
      show_terminal: false,
      show_gpu_info: false,
      show_memory_stat: false,
    }
  }
}

impl Widget for Viewer {
  #[instrument(name = "viewer update state", skip_all)]
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
        // todo, fix , this should use actual render resolution instead of full window size
        let canvas_resolution = Vec2::new(
          input.window_state.physical_size.0 / input.window_state.device_pixel_ratio,
          input.window_state.physical_size.1 / input.window_state.device_pixel_ratio,
        )
        .map(|v| v.ceil() as u32);

        let mut widget_derive_access = Box::new(WidgetEnvAccessImpl {
          world_mat: derived.world_mat.clone(),
          camera_node: viewer_scene.camera_node,
          camera_proj: scene_reader
            .camera
            .read::<SceneCameraPerspective>(viewer_scene.main_camera)
            .unwrap(),
          canvas_resolution,
          camera_world_ray: picker.current_mouse_ray_in_world(),
          normalized_canvas_position: picker.normalized_position_ndc(),
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
  #[instrument(name = "viewer update view", skip_all)]
  fn update_view(&mut self, cx: &mut DynCx) {
    access_cx!(cx, platform, PlatformEventInput);
    let size = platform.window_state.physical_size;
    let size_changed = platform.state_delta.size_change;
    if size_changed {
      self.rendering.resize_view()
    }

    let scene = self.scene.scene;
    cx.scoped_cx(&mut self.scene, |cx| {
      cx.scoped_cx(&mut self.derives, |cx| {
        cx.split_cx::<egui::Context>(|egui_cx, cx| {
          egui(
            &mut self.terminal,
            &mut self.background,
            &mut self.on_demand_rendering,
            &mut self.rendering,
            scene,
            egui_cx,
            &mut self.ui_state,
            cx,
          );
          crate::db_egui_view::egui_db_gui(
            egui_cx,
            &mut self.egui_db_inspector,
            &mut self.ui_state.show_db_inspector,
          );
        });
      });

      noop_ctx!(ctx);
      self.camera_helpers.prepare_update(ctx);

      let time = Instant::now()
        .duration_since(self.started_time)
        .as_secs_f32();

      access_cx!(cx, viewer_scene, Viewer3dSceneCtx);
      let mutation = self
        .animation_player
        .compute_mutation(ctx, viewer_scene.scene, time);

      let mut writer = SceneWriter::from_global(viewer_scene.scene);

      mutation.apply(&mut writer);

      self.camera_helpers.apply_updates(
        &mut writer,
        viewer_scene.widget_scene,
        viewer_scene.main_camera,
      );

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
    self.rendering.tick_frame();
  }

  fn clean_up(&mut self, cx: &mut DynCx) {
    let mut writer = SceneWriter::from_global(self.scene.scene);
    cx.scoped_cx(&mut writer, |cx| self.content.clean_up(cx));
    self.camera_helpers.do_cleanup(&mut writer);
  }
}

impl Viewer {
  pub fn new(
    gpu: GPU,
    swap_chain: ApplicationWindowSurface,
    content_logic: impl Widget + 'static,
  ) -> Self {
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

    let background = {
      let mut writer = SceneWriter::from_global(scene.scene);
      load_default_scene(&mut writer, &scene);

      ViewerBackgroundState::init(&mut writer)
    };

    let viewer_ndc = ViewerNDC {
      enable_reverse_z: true,
    };

    let camera_transforms = camera_transforms(viewer_ndc)
      .into_boxed()
      .into_static_forker();

    let derives = Viewer3dSceneDeriveSource {
      world_mat: Box::new(scene_node_derive_world_mat()),
      node_net_visible: Box::new(scene_node_derive_visible()),
      camera_transforms: camera_transforms.clone().into_boxed(),
      mesh_vertex_ref: Box::new(
        global_rev_ref()
          .watch_inv_ref::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>(),
      ),
      sm_to_s: Box::new(global_rev_ref().watch_inv_ref::<SceneModelBelongsToScene>()),
      mesh_local_bounding: Box::new(attribute_mesh_local_bounding()),
      node_children: Box::new(scene_node_connectivity_many_one_relation()),
    };

    let camera_helpers = SceneCameraHelper::new(scene.scene, camera_transforms.clone());

    Self {
      widget_intersection_group: Default::default(),
      // todo, we current disable the on demand draw
      // because we not cache the rendering result yet
      on_demand_rendering: false,
      ui_state: ViewerUIState::default(),
      content: Box::new(content_logic),
      camera_helpers,
      scene,
      terminal,
      rendering: Viewer3dRenderingCtx::new(gpu, swap_chain, viewer_ndc, camera_transforms),
      derives,
      on_demand_draw: Default::default(),
      egui_db_inspector: Default::default(),
      background,
      animation_player: SceneAnimationsPlayer::new(),
      started_time: Instant::now(),
    }
  }

  pub fn draw_canvas(&mut self, canvas: &RenderTargetView) {
    if !self.on_demand_rendering {
      self.on_demand_draw.wake();
    }

    self.on_demand_draw.update_once(|cx| {
      // println!("draw");
      self.rendering.render(canvas, &self.scene, cx)
    });
  }
}

fn egui(
  terminal: &mut Terminal,
  background: &mut ViewerBackgroundState,
  on_demand_rendering: &mut bool,
  rendering: &mut Viewer3dRenderingCtx,
  scene: EntityHandle<SceneEntity>,
  ui: &egui::Context,
  ui_state: &mut ViewerUIState,
  cx: &mut DynCx,
) {
  egui::TopBottomPanel::top("view top menu").show(ui, |ui| {
    ui.horizontal_wrapped(|ui| {
      egui::widgets::global_theme_preference_switch(ui);
      ui.separator();
      ui.checkbox(&mut ui_state.show_db_inspector, "database inspector");
      ui.checkbox(&mut ui_state.show_viewer_config_panel, "viewer config");
      ui.checkbox(&mut ui_state.show_terminal, "terminal");
      ui.checkbox(&mut ui_state.show_gpu_info, "gpu info");
      ui.checkbox(&mut ui_state.show_memory_stat, "heap stat");
    });
  });

  egui::Window::new("Memory")
    .vscroll(true)
    .open(&mut ui_state.show_memory_stat)
    .show(ui, |ui| {
      #[cfg(feature = "heap-debug")]
      {
        if ui.button("reset heap peak stat").clicked() {
          GLOBAL_ALLOCATOR.reset_history_peak();
        }
        let stat = GLOBAL_ALLOCATOR.report();
        ui.label(format!("{:#?}", stat));
        if ui.button("reset counter peak stat").clicked() {
          heap_tools::HEAP_TOOL_GLOBAL_INSTANCE_COUNTER
            .write()
            .reset_all_instance_history_peak();
        }
        let global = heap_tools::HEAP_TOOL_GLOBAL_INSTANCE_COUNTER.read();
        for (ty, report) in global.report_all_instance_count() {
          ui.label(format!(
            "{ty} => current: {}, peak: {}",
            report.current, report.history_peak,
          ));
        }
      }
      #[cfg(not(feature = "heap-debug"))]
      {
        ui.label("heap stat is not enabled in this build");
      }
    });

  egui::Window::new("Viewer")
    .vscroll(true)
    .open(&mut ui_state.show_viewer_config_panel)
    .default_pos([10., 60.])
    .max_width(1000.0)
    .max_height(800.0)
    .default_width(250.0)
    .default_height(400.0)
    .resizable(true)
    .movable(true)
    .show(ui, |ui| {
      ui.checkbox(on_demand_rendering, "enable on demand rendering");
      ui.separator();
      rendering.egui(ui);
      ui.separator();

      background.egui(ui, scene);

      ui.separator();

      ui.collapsing("Instance Counts", |ui| {
        let mut counters = heap_tools::HEAP_TOOL_GLOBAL_INSTANCE_COUNTER.write();

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

  egui::Window::new("GPU Info")
    .open(&mut ui_state.show_gpu_info)
    .vscroll(true)
    .show(ui, |ui| {
      let info = &rendering.gpu().info;

      ui.collapsing("adaptor info", |ui| {
        ui.label(format!("{:#?}", info.adaptor_info));
        ui.label(format!("power preference: {:?}", info.power_preference));
      });

      ui.collapsing("supported_features", |ui| {
        for (supported_feature, _) in info.supported_features.iter_names() {
          ui.label(format!("{:?}", supported_feature));
        }
      });

      ui.collapsing("supported limits", |ui| {
        ui.label(format!("{:#?}", info.supported_limits));
      });
    });

  if ui_state.show_terminal {
    egui::TopBottomPanel::bottom("view bottom terminal")
      .resizable(true)
      .show(ui, |ui| {
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
    noop_ctx!(cx);

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
