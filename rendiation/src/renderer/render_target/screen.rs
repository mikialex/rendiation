use crate::{WGPURenderer, WGPUTexture, TargetStates, TargetStatesProvider, RenderTargetAble, WGPURenderPassBuilder};

pub struct ScreenRenderTarget {
  swap_chain_format: wgpu::TextureFormat,
  depth: Option<WGPUTexture>,
}

impl ScreenRenderTarget {
  pub fn resize(&mut self, renderer: &WGPURenderer, size: (usize, usize)) {
    if let Some(depth) = &mut self.depth {
      depth.resize(renderer, size)
    }
  }
}


impl TargetStatesProvider for ScreenRenderTarget {
  fn create_target_states(&self) -> TargetStates {
    let color_states = vec![wgpu::ColorStateDescriptor {
      format: self.swap_chain_format,
      color_blend: wgpu::BlendDescriptor::REPLACE,
      alpha_blend: wgpu::BlendDescriptor::REPLACE,
      write_mask: wgpu::ColorWrite::ALL,
    }];

    let depth_state = self
      .depth
      .as_ref()
      .map(|d| wgpu::DepthStencilStateDescriptor {
        format: *d.format(),
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
        stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
        stencil_read_mask: 0,
        stencil_write_mask: 0,
      });

    TargetStates {
      color_states,
      depth_state,
    }
  }
}

impl ScreenRenderTarget {
  pub fn new(swap_chain_format: wgpu::TextureFormat, depth: Option<WGPUTexture>) -> Self {
    Self {
      swap_chain_format,
      depth,
    }
  }

  pub fn create_instance<'a>(
    &'a mut self,
    swap_chain_view: &'a wgpu::TextureView,
  ) -> ScreenRenderTargetInstance<'a> {
    ScreenRenderTargetInstance {
      swap_chain_view,
      base: self,
    }
  }
}

pub struct ScreenRenderTargetInstance<'a> {
  swap_chain_view: &'a wgpu::TextureView,
  base: &'a mut ScreenRenderTarget,
}
impl<'a> TargetStatesProvider for ScreenRenderTargetInstance<'a> {
  fn create_target_states(&self) -> TargetStates {
    self.base.create_target_states()
  }
}
impl<'a> RenderTargetAble for ScreenRenderTargetInstance<'a> {
  fn create_render_pass_builder(&self) -> WGPURenderPassBuilder {
    let attachments = vec![wgpu::RenderPassColorAttachmentDescriptor {
      attachment: self.swap_chain_view,
      resolve_target: None,
      load_op: wgpu::LoadOp::Load,
      store_op: wgpu::StoreOp::Store,
      clear_color: wgpu::Color {
        r: 0.,
        g: 0.,
        b: 0.,
        a: 1.,
      },
    }];

    let depth =
      self
        .base
        .depth
        .as_ref()
        .map(|d| wgpu::RenderPassDepthStencilAttachmentDescriptor {
          attachment: d.view(),
          depth_load_op: wgpu::LoadOp::Clear,
          depth_store_op: wgpu::StoreOp::Store,
          stencil_load_op: wgpu::LoadOp::Clear,
          stencil_store_op: wgpu::StoreOp::Store,
          clear_depth: 1.0,
          clear_stencil: 0,
        });
    WGPURenderPassBuilder { attachments, depth }
  }
  fn resize(&mut self, renderer: &WGPURenderer, size: (usize, usize)) {
    self.base.resize(renderer, size)
  }
}
