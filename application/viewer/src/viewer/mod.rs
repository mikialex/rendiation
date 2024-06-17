use crate::*;

mod feature;
mod pick;
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
        cx.scoped_cx(&mut self.rendering, |cx| {
          self.content.update_state(cx);
        });
      });
    });
  }
  fn update_view(&mut self, cx: &mut DynCx) {
    cx.scoped_cx(&mut self.scene, |cx| {
      cx.scoped_cx(&mut self.rendering, |cx| {
        self.content.update_view(cx);
      });
    });

    cx.split_cx::<egui::Context>(|egui_cx, cx| {
      self.egui(egui_cx, cx);
    });

    access_cx!(cx, draw_target_canvas, RenderTargetView);
    self.draw_canvas(draw_target_canvas);
  }

  fn clean_up(&mut self, cx: &mut DynCx) {
    self.content.clean_up(cx)
  }
}

impl Viewer {
  pub fn new(gpu: Arc<GPU>, content_logic: impl Widget + 'static) -> Self {
    let mut terminal = Terminal::default();
    register_default_commands(&mut terminal);

    let scene = global_entity_of::<SceneEntity>()
      .entity_writer()
      .new_entity();
    let main_camera = global_entity_of::<SceneCameraEntity>()
      .entity_writer()
      .new_entity();

    let scene = Viewer3dSceneCtx {
      main_camera,
      scene,
      selected_target: None,
    };

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
      .default_width(800.0)
      .resizable(true)
      .movable(true)
      .anchor(egui::Align2::LEFT_TOP, [3.0, 3.0])
      .show(ui, |ui| {
        if ui.add(egui::Button::new("Click me")).clicked() {
          println!("PRESSED")
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
  pub scene: EntityHandle<SceneEntity>,
  pub selected_target: Option<EntityHandle<SceneModelEntity>>,
}

pub struct Viewer3dSceneDeriveSource {
  pub world_mat: Box<dyn ReactiveCollection<EntityHandle<SceneNodeEntity>, Mat4<f32>>>,
  pub camera_proj: Box<dyn ReactiveCollection<EntityHandle<SceneCameraEntity>, Mat4<f32>>>,
}

impl Viewer3dSceneDeriveSource {
  fn poll_update(&self, cx: &mut Context) -> Viewer3dSceneDerive {
    let _ = self.world_mat.poll_changes(cx);
    let _ = self.camera_proj.poll_changes(cx);
    Viewer3dSceneDerive {
      world_mat: self.world_mat.access(),
      camera_proj: self.camera_proj.access(),
    }
  }
}

/// used in render & scene update
pub struct Viewer3dSceneDerive {
  pub world_mat: Box<dyn VirtualCollection<EntityHandle<SceneNodeEntity>, Mat4<f32>>>,
  pub camera_proj: Box<dyn VirtualCollection<EntityHandle<SceneCameraEntity>, Mat4<f32>>>,
}

pub struct Viewer3dSceneCtxWriterWidget<V>(pub V);

impl<V: Widget> Widget for Viewer3dSceneCtxWriterWidget<V> {
  fn update_state(&mut self, cx: &mut DynCx) {
    self.0.update_state(cx)
  }

  fn update_view(&mut self, cx: &mut DynCx) {
    access_cx!(cx, viewer_scene, Viewer3dSceneCtx);
    let mut writer = Scene3dWriter::from_global(viewer_scene.scene);
    cx.scoped_cx(&mut writer, |cx| {
      self.0.update_view(cx);
    })
  }

  fn clean_up(&mut self, cx: &mut DynCx) {
    self.0.clean_up(cx)
  }
}
