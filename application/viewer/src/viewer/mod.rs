use crate::*;

mod feature;
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
  content: Box<dyn Widget>,
  terminal: Terminal,
}

impl Widget for Viewer {
  fn update_state(&mut self, cx: &mut StateCx) {
    // todo, update camera view size
    state_access!(cx, platform, PlatformEventInput);
    if platform.state_delta.size_change {
      self.rendering.resize_view()
    }

    cx.state_scope(&mut self.scene, |cx| {
      cx.state_scope(&mut self.rendering, |cx| {
        self.content.update_state(cx);
      });
    });
  }
  fn update_view(&mut self, cx: &mut StateCx) {
    state_access!(cx, draw_target_canvas, RenderTargetView);
    self.draw_canvas(draw_target_canvas);

    cx.state_scope(&mut self.scene, |cx| {
      cx.state_scope(&mut self.rendering, |cx| {
        self.content.update_view(cx);
      });
    });

    cx.split_state::<egui::Context>(|egui_cx, cx| {
      self.egui(egui_cx, cx);
    });
  }

  fn clean_up(&mut self, cx: &mut StateCx) {
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

    let scene = Viewer3dSceneCtx { main_camera, scene };

    Self {
      // todo, we current disable the on demand draw
      // because we not cache the rendering result yet
      on_demand_rendering: false,
      content: Box::new(content_logic),
      scene,
      terminal,
      rendering: Viewer3dRenderingCtx::new(gpu),
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

  pub fn egui(&mut self, ui: &egui::Context, cx: &mut StateCx) {
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
}

pub struct Viewer3dSceneCtxWriterWidget<V>(pub V);

impl<V: Widget> Widget for Viewer3dSceneCtxWriterWidget<V> {
  fn update_state(&mut self, cx: &mut StateCx) {
    self.0.update_state(cx)
  }

  fn update_view(&mut self, cx: &mut StateCx) {
    state_access!(cx, viewer_scene, Viewer3dSceneCtx);
    let mut writer = Scene3dWriter::from_global(viewer_scene.scene);
    cx.state_scope(&mut writer, |cx| {
      self.0.update_view(cx);
    })
  }

  fn clean_up(&mut self, cx: &mut StateCx) {
    self.0.clean_up(cx)
  }
}
