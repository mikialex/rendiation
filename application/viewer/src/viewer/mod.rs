use crate::*;

mod feature;
mod pick;
use default_scene::load_default_scene;
pub use feature::*;

mod terminal;
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
  egui_db_inspector: egui_db::DBInspector,
  terminal: Terminal,
}

impl Widget for Viewer {
  fn update_state(&mut self, cx: &mut DynCx) {
    // todo, update camera view size
    access_cx!(cx, platform, PlatformEventInput);
    if platform.state_delta.size_change {
      self.rendering.resize_view()
    }
    let waker = futures::task::noop_waker_ref();
    let mut ctx = Context::from_waker(waker);
    let mut derived = self.derives.poll_update(&mut ctx);

    cx.scoped_cx(&mut derived, |cx| {
      cx.scoped_cx(&mut self.scene, |cx| {
        access_cx!(cx, viewer_scene, Viewer3dSceneCtx);
        let mut writer = Scene3dWriter::from_global(viewer_scene.scene);
        cx.scoped_cx(&mut writer, |cx| {
          cx.scoped_cx(&mut self.rendering, |cx| {
            self.content.update_state(cx);
          });
        });
      });
    });
  }
  fn update_view(&mut self, cx: &mut DynCx) {
    cx.split_cx::<egui::Context>(|egui_cx, cx| {
      self.egui(egui_cx, cx);
      crate::egui_db::egui_db_gui(egui_cx, &mut self.egui_db_inspector);
    });

    cx.scoped_cx(&mut self.scene, |cx| {
      access_cx!(cx, viewer_scene, Viewer3dSceneCtx);
      let mut writer = Scene3dWriter::from_global(viewer_scene.scene);
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
        Vec3::new(10., 10., 10.),
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
      let mut writer = Scene3dWriter::from_global(scene.scene);
      load_default_scene(&mut writer, &scene);
    }

    let derives = Viewer3dSceneDeriveSource {
      world_mat: Box::new(scene_node_derive_world_mat()),
      camera_proj: Box::new(camera_project_matrix()),
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
  pub camera_proj: Box<dyn DynReactiveCollection<EntityHandle<SceneCameraEntity>, Mat4<f32>>>,
}

impl Viewer3dSceneDeriveSource {
  fn poll_update(&self, cx: &mut Context) -> Viewer3dSceneDerive {
    let (_, world_mat) = self.world_mat.poll_changes(cx);
    let (_, camera_proj) = self.camera_proj.poll_changes(cx);
    Viewer3dSceneDerive {
      world_mat,
      camera_proj,
    }
  }
}

/// used in render & scene update
pub struct Viewer3dSceneDerive {
  pub world_mat: Box<dyn DynVirtualCollection<EntityHandle<SceneNodeEntity>, Mat4<f32>>>,
  pub camera_proj: Box<dyn DynVirtualCollection<EntityHandle<SceneCameraEntity>, Mat4<f32>>>,
}
