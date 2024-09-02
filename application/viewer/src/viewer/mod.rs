use crate::*;

mod feature;
mod pick;
use default_scene::load_default_scene;
pub use feature::*;

mod terminal;
use pick::*;
pub use terminal::*;

mod rendering;
pub use rendering::*;

pub struct Viewer {
  on_demand_rendering: bool,
  on_demand_draw: NotifyScope,
  scene: Viewer3dSceneCtx,
  rendering: Viewer3dRenderingCtx,
  derives: Viewer3dSceneDeriveSource,
  content: Box<dyn Widget>,
  egui_db_inspector: db_egui_view::DBInspector,
  terminal: Terminal,
}

impl Widget for Viewer {
  fn update_state(&mut self, cx: &mut DynCx) {
    let waker = futures::task::noop_waker_ref();
    let mut ctx = Context::from_waker(waker);
    let mut derived = self.derives.poll_update(&mut ctx);

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

        // todo, scene3d reader
        // let mut writer = Scene3dWriter::from_global(viewer_scene.scene);
        // cx.scoped_cx(&mut writer, |cx| {
        cx.scoped_cx(&mut self.rendering, |cx| {
          self.content.update_state(cx);
        });
        // });
      });
    });
  }
  fn update_view(&mut self, cx: &mut DynCx) {
    cx.split_cx::<egui::Context>(|egui_cx, cx| {
      self.egui(egui_cx, cx);
      crate::db_egui_view::egui_db_gui(egui_cx, &mut self.egui_db_inspector);
    });

    access_cx!(cx, platform, PlatformEventInput);
    let size = platform.window_state.size;
    let size_changed = platform.state_delta.size_change;
    if size_changed {
      self.rendering.resize_view()
    }

    cx.scoped_cx(&mut self.scene, |cx| {
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
    self.content.clean_up(cx)
  }
}

impl Viewer {
  pub fn new(gpu: GPU, content_logic: impl Widget + 'static) -> Self {
    let mut terminal = Terminal::default();
    register_default_commands(&mut terminal);

    let scene = global_entity_of::<SceneEntity>()
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
    };

    Self {
      // todo, we current disable the on demand draw
      // because we not cache the rendering result yet
      on_demand_rendering: false,
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

  pub fn egui(&mut self, ui: &egui::Context, cx: &mut DynCx) {
    egui::Window::new("Viewer")
      .vscroll(true)
      .default_open(true)
      .default_pos([10., 60.])
      .max_width(1000.0)
      .max_height(800.0)
      .default_width(400.0)
      .default_height(300.0)
      .resizable(true)
      .movable(true)
      .show(ui, |ui| {
        if ui.add(egui::Button::new("Click me")).clicked() {
          println!("PRESSED")
        }
        if ui.button("Organize windows").clicked() {
          ui.ctx().memory_mut(|mem| mem.reset_areas());
        }

        ui.separator();
        ui.checkbox(&mut self.on_demand_rendering, "enable on demand rendering");
        ui.separator();
        self.rendering.pipeline.egui(ui);
        ui.separator();
        self.terminal.egui(ui, cx);
      });
  }
}

pub struct Viewer3dSceneCtx {
  pub main_camera: EntityHandle<SceneCameraEntity>,
  pub camera_node: EntityHandle<SceneNodeEntity>,
  pub root: EntityHandle<SceneNodeEntity>,
  pub scene: EntityHandle<SceneEntity>,
  pub selected_target: Option<EntityHandle<SceneModelEntity>>,
}

pub struct Viewer3dSceneDeriveSource {
  pub world_mat: Box<dyn DynReactiveCollection<EntityHandle<SceneNodeEntity>, Mat4<f32>>>,
  pub node_net_visible: Box<dyn DynReactiveCollection<EntityHandle<SceneNodeEntity>, bool>>,
  pub camera_transforms:
    Box<dyn DynReactiveCollection<EntityHandle<SceneCameraEntity>, CameraTransform>>,
  pub mesh_vertex_ref:
    RevRefOfForeignKeyWatch<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>,
}

impl Viewer3dSceneDeriveSource {
  fn poll_update(&self, cx: &mut Context) -> Viewer3dSceneDerive {
    let (_, world_mat) = self.world_mat.poll_changes(cx);
    let (_, node_net_visible) = self.node_net_visible.poll_changes(cx);
    let (_, camera_transforms) = self.camera_transforms.poll_changes(cx);
    let (_, _, mesh_vertex_ref) = self.mesh_vertex_ref.poll_changes_with_inv_dyn(cx);
    Viewer3dSceneDerive {
      world_mat,
      camera_transforms,
      mesh_vertex_ref,
      node_net_visible,
    }
  }
}

/// used in render & scene update
pub struct Viewer3dSceneDerive {
  pub world_mat: Box<dyn DynVirtualCollection<EntityHandle<SceneNodeEntity>, Mat4<f32>>>,
  pub node_net_visible: Box<dyn DynVirtualCollection<EntityHandle<SceneNodeEntity>, bool>>,
  pub camera_transforms:
    Box<dyn DynVirtualCollection<EntityHandle<SceneCameraEntity>, CameraTransform>>,
  pub mesh_vertex_ref:
    RevRefOfForeignKey<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>,
}
