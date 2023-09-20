use std::sync::Arc;

mod content;
pub use content::*;

mod default_scene;
pub use default_scene::*;
mod rendering;
use futures::Stream;
use reactive::NotifyScope;
pub use rendering::*;

mod controller;
pub use controller::*;
mod selection;
pub use selection::*;

mod helpers;
pub use helpers::*;
use interphaser::*;
use rendiation_texture::Size;
use webgpu::*;

use crate::*;

pub struct Viewer {
  content: Viewer3dContent,
  ctx: Option<Viewer3dRenderingCtx>,
  size: Size,
  pub terminal: Terminal,
  pub io_executor: futures::executor::ThreadPool,
  pub compute_executor: rayon::ThreadPool,
  pub on_demand_draw: NotifyScope,
}

impl Viewer {
  pub fn new(terminal_inputs: impl Stream<Item = String> + Unpin + 'static) -> Self {
    let io_executor = futures::executor::ThreadPool::builder()
      .name_prefix("rendiation_io_threads")
      .pool_size(2)
      .create()
      .unwrap();

    let compute_executor = rayon::ThreadPoolBuilder::new()
      .thread_name(|i| format!("rendiation_compute_threads-{i}"))
      .build()
      .unwrap();

    let mut viewer = Self {
      content: Viewer3dContent::new(),
      size: Size::from_u32_pair_min_one((100, 100)),
      terminal: Terminal::new(terminal_inputs),
      ctx: None,
      io_executor,
      compute_executor,
      on_demand_draw: Default::default(),
    };

    register_default_commands(&mut viewer.terminal);

    viewer
  }
}

impl CanvasPrinter for Viewer {
  fn draw_canvas(&mut self, gpu: &Arc<GPU>, canvas: GPU2DTextureView) {
    self.on_demand_draw.notify_by(|cx| {
      self.content.poll_update(cx);
      self.content.per_frame_update();
      if let Some(ctx) = &mut self.ctx {
        ctx.setup_render_waker(cx);
      }
      self.content.poll_update(cx);
    });

    self.on_demand_draw.update_once(|cx| {
      println!("draw");
      self
        .ctx
        .get_or_insert_with(|| Viewer3dRenderingCtx::new(gpu.clone()))
        .render(RenderTargetView::Texture(canvas), &mut self.content, cx)
    });
  }

  fn event(
    &mut self,
    event: &winit::event::Event<()>,
    states: &WindowState,
    position_info: CanvasWindowPositionInfo,
  ) {
    self.on_demand_draw.notify_by(|cx| {
      let mut ctx = CommandCtx {
        scene: &self.content.scene,
        rendering: self.ctx.as_mut(),
        selection_set: &self.content.selections,
      };

      self.terminal.check_execute(&mut ctx, cx);
      self.content.poll_update(cx);
      self.content.per_event_update(event, states, position_info)
    });
  }

  fn update_render_size(&mut self, layout_size: (f32, f32)) -> Size {
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
