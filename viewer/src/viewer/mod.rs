use std::sync::Arc;

mod content;
pub use content::*;

mod default_scene;
pub use default_scene::*;
mod rendering;
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

pub struct ViewerImpl {
  content: Viewer3dContent,
  ctx: Option<Viewer3dRenderingCtx>,
  size: Size,
  pub terminal: Terminal,
  pub io_executor: futures::executor::ThreadPool,
  pub compute_executor: rayon::ThreadPool,
}

impl Default for ViewerImpl {
  fn default() -> Self {
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
      terminal: Default::default(),
      ctx: None,
      io_executor,
      compute_executor,
    };

    register_default_commands(&mut viewer.terminal);

    viewer
  }
}

impl CanvasPrinter for ViewerImpl {
  fn draw_canvas(&mut self, gpu: &Arc<GPU>, canvas: GPU2DTextureView) {
    self.content.update_state();
    self
      .ctx
      .get_or_insert_with(|| Viewer3dRenderingCtx::new(gpu.clone()))
      .render(RenderTargetView::Texture(canvas), &mut self.content)
  }

  fn event(
    &mut self,
    event: &winit::event::Event<()>,
    states: &WindowState,
    position_info: CanvasWindowPositionInfo,
  ) {
    let mut ctx = CommandCtx {
      scene: &self.content.scene,
      rendering: self.ctx.as_mut(),
    };

    self.terminal.check_execute(&mut ctx);
    self.content.event(event, states, position_info)
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
