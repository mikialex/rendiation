use crate::*;
// mod content;
// pub use content::*;

mod feature;
pub use feature::*;

mod terminal;
use rendiation_gui_3d::{state_access, StateCx, Widget};
pub use terminal::*;

mod rendering;
use reactive::NotifyScope;
pub use rendering::*;
use rendiation_texture_core::Size;
use rendiation_webgpu::*;

pub struct Viewer3dSceneContext {
  pub main_camera: AllocIdx<SceneCameraEntity>,
  pub scene: AllocIdx<SceneEntity>,
}

pub struct Viewer {
  on_demand_rendering: bool,
  on_demand_draw: NotifyScope,
  scene: Viewer3dSceneContext,
  content: Box<dyn Widget>,
  terminal: Terminal,
  ctx: Viewer3dRenderingCtx,
  size: Size,
}

impl Widget for Viewer {
  fn update_state(&mut self, cx: &mut StateCx) {
    // todo, update size
    self.content.update_state(cx)
  }
  fn update_view(&mut self, cx: &mut StateCx) {
    state_access!(cx, draw_target_canvas, RenderTargetView);
    self.draw_canvas(draw_target_canvas);

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
    Self {
      // todo, we current disable the on demand draw
      // because we not cache the rendering result yet
      on_demand_rendering: false,
      content: Box::new(content_logic),
      scene: todo!(),
      size: Size::from_u32_pair_min_one((100, 100)),
      terminal: Default::default(),
      ctx: Viewer3dRenderingCtx::new(gpu),
      on_demand_draw: Default::default(),
    }
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
        self.ctx.pipeline.egui(ui);
        ui.separator();
        self.terminal.egui(ui, cx);
      });
  }

  pub fn draw_canvas(&mut self, canvas: &RenderTargetView) {
    if !self.on_demand_rendering {
      self.on_demand_draw.wake();
    }

    self.on_demand_draw.update_once(|cx| {
      // println!("draw");
      self.ctx.render(canvas.clone(), &self.scene, cx)
    });
  }

  pub fn update_render_size(&mut self, layout_size: (f32, f32)) -> Size {
    let new_size = (layout_size.0 as u32, layout_size.1 as u32);
    let new_size = Size::from_u32_pair_min_one(new_size);

    self.size = new_size;
    new_size
  }
}
