use crate::*;
mod content;
pub use content::*;

mod terminal;
use rendiation_gui_3d::{state_access, StateCx, StatefulView};
pub use terminal::*;

mod default_scene;
// pub use default_scene::*;
mod rendering;
use reactive::{EventSource, NotifyScope};
pub use rendering::*;
use rendiation_texture_core::Size;
use rendiation_webgpu::*;

pub struct Viewer {
  content: Viewer3dContent,
  pub terminal: Terminal,
  pub(crate) ctx: Viewer3dRenderingCtx,
  size: Size,
  pub io_executor: futures::executor::ThreadPool,
  pub compute_executor: rayon::ThreadPool,
  pub on_demand_draw: NotifyScope,
}

impl StatefulView for Viewer {
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
  pub fn new(gpu: Arc<GPU>) -> Self {
    let io_executor = futures::executor::ThreadPool::builder()
      .name_prefix("viewer_io_threads")
      .pool_size(2)
      .create()
      .unwrap();

    let compute_executor = rayon::ThreadPoolBuilder::new()
      .thread_name(|i| format!("viewer_compute_threads-{i}"))
      .build()
      .unwrap();

    Self {
      content: Viewer3dContent::new(),
      size: Size::from_u32_pair_min_one((100, 100)),
      terminal: Default::default(),
      ctx: Viewer3dRenderingCtx::new(gpu),
      io_executor,
      compute_executor,
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

        self.ctx.pipeline.egui(ui);

        self.terminal.egui(ui, cx, &self.io_executor);
      });
  }

  pub fn draw_canvas(&mut self, canvas: &RenderTargetView) {
    self.on_demand_draw.notify_by(|cx| {
      self.content.per_frame_update();
    });

    self.on_demand_draw.wake(); // todo, we current disable the on demand draw
                                // because we not cache the rendering result yet
    self.on_demand_draw.update_once(|cx| {
      // println!("draw");
      self.ctx.render(canvas.clone(), &mut self.content, cx)
    });
  }

  pub fn update_render_size(&mut self, layout_size: (f32, f32)) -> Size {
    let new_size = (layout_size.0 as u32, layout_size.1 as u32);
    let new_size = Size::from_u32_pair_min_one(new_size);

    self.size = new_size;
    new_size
  }
}
