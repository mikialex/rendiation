use std::sync::Arc;

mod content;
pub use content::*;

mod terminal;
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
  pub(crate) ctx: Option<Viewer3dRenderingCtx>,
  size: Size,
  pub terminal: Terminal,
  pub terminal_input: EventSource<String>,
  pub io_executor: futures::executor::ThreadPool,
  pub compute_executor: rayon::ThreadPool,
  pub on_demand_draw: NotifyScope,
}

impl Default for Viewer {
  fn default() -> Self {
    let io_executor = futures::executor::ThreadPool::builder()
      .name_prefix("viewer_io_threads")
      .pool_size(2)
      .create()
      .unwrap();

    let compute_executor = rayon::ThreadPoolBuilder::new()
      .thread_name(|i| format!("viewer_compute_threads-{i}"))
      .build()
      .unwrap();

    let terminal_input = EventSource::<String>::default();
    let command_stream = terminal_input.unbound_listen();

    Self {
      content: Viewer3dContent::new(),
      size: Size::from_u32_pair_min_one((100, 100)),
      terminal_input,
      terminal: Terminal::new(command_stream),
      ctx: None,
      io_executor,
      compute_executor,
      on_demand_draw: Default::default(),
    }
  }
}

impl Viewer {
  pub fn draw_canvas(&mut self, gpu: &Arc<GPU>, canvas: RenderTargetView) {
    self.on_demand_draw.notify_by(|cx| {
      self.content.poll_update(cx);
      self.content.per_frame_update();
      if let Some(ctx) = &mut self.ctx {
        ctx.setup_render_waker(cx);
      }
      self.content.poll_update(cx);
    });

    self.on_demand_draw.wake(); // todo, we current disable the on demand draw
                                // because we not cache the rendering result yet
    self.on_demand_draw.update_once(|cx| {
      // println!("draw");
      self
        .ctx
        .get_or_insert_with(|| Viewer3dRenderingCtx::new(gpu.clone()))
        .render(canvas, &mut self.content, cx)
    });
  }

  pub fn event(
    &mut self,
    event: &winit::event::Event<()>,
    states: &WindowState,
    position_info: CanvasWindowPositionInfo,
  ) {
    self.on_demand_draw.notify_by(|cx| {
      let mut ctx = CommandCtx {
        rendering: self.ctx.as_mut(),
      };

      self.terminal.check_execute(&mut ctx, cx, &self.io_executor);
      self.content.poll_update(cx);
      self.content.per_event_update(event, states, position_info)
    });
  }

  pub fn update_render_size(&mut self, layout_size: (f32, f32)) -> Size {
    let new_size = (layout_size.0 as u32, layout_size.1 as u32);
    let new_size = Size::from_u32_pair_min_one(new_size);
    if let Some(ctx) = &mut self.ctx {
      if self.size != new_size {
        ctx.resize_view();
        self.content.resize_view(layout_size);
      }
    }
    self.size = new_size;
    new_size
  }
}

pub trait ViewerFeature {
  fn setup(&self, viewer: &mut Viewer);
  fn dispose(&mut self, viewer: &mut Viewer);
}
