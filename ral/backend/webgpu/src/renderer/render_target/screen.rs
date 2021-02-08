use rendiation_ral::{RenderTargetFormatsInfo, TargetStates};

use crate::{
  RenderTargetAble, TargetInfoProvider, WGPURenderPassBuilder, WGPURenderer, WGPUTexture,
};

pub struct ScreenRenderTarget {
  size: (usize, usize),
  swap_chain_format: wgpu::TextureFormat,
  depth: Option<WGPUTexture>,
}

impl ScreenRenderTarget {
  pub fn resize(&mut self, renderer: &WGPURenderer, size: (usize, usize)) {
    if let Some(depth) = &mut self.depth {
      depth.resize(renderer, size)
    }
    self.size = size;
  }
}

impl TargetInfoProvider for ScreenRenderTarget {
  fn create_target_states(&self) -> TargetStates {
    let color_states = vec![wgpu::ColorTargetState {
      format: self.swap_chain_format,
      color_blend: wgpu::BlendState::REPLACE,
      alpha_blend: wgpu::BlendState::REPLACE,
      write_mask: wgpu::ColorWrite::ALL,
    }];

    let depth_state = self.depth.as_ref().map(|d| wgpu::DepthStencilState {
      format: *d.format(),
      depth_write_enabled: true,
      depth_compare: wgpu::CompareFunction::LessEqual,
      stencil: wgpu::StencilState::default(),
      bias: wgpu::DepthBiasState::default(),
      clamp_depth: false,
    });

    TargetStates {
      color_states,
      depth_state,
    }
  }
  fn provide_format_info(&self) -> RenderTargetFormatsInfo {
    let color = vec![self.swap_chain_format];
    let depth = self.depth.as_ref().map(|d| *d.format());
    RenderTargetFormatsInfo { color, depth }
  }
}

impl ScreenRenderTarget {
  pub fn new(
    swap_chain_format: wgpu::TextureFormat,
    depth: Option<WGPUTexture>,
    size: (usize, usize),
  ) -> Self {
    Self {
      swap_chain_format,
      depth,
      size,
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
impl<'a> TargetInfoProvider for ScreenRenderTargetInstance<'a> {
  fn create_target_states(&self) -> TargetStates {
    self.base.create_target_states()
  }
  fn provide_format_info(&self) -> RenderTargetFormatsInfo {
    self.base.provide_format_info()
  }
}
impl<'a> RenderTargetAble for ScreenRenderTargetInstance<'a> {
  fn create_render_pass_builder(&self) -> WGPURenderPassBuilder {
    let attachments = vec![wgpu::RenderPassColorAttachmentDescriptor {
      attachment: self.swap_chain_view,
      resolve_target: None,
      ops: wgpu::Operations {
        load: wgpu::LoadOp::Load,
        store: true,
      },
    }];

    let depth =
      self
        .base
        .depth
        .as_ref()
        .map(|d| wgpu::RenderPassDepthStencilAttachmentDescriptor {
          attachment: d.view(),
          depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: true,
          }),
          stencil_ops: None,
        });
    WGPURenderPassBuilder {
      attachments,
      depth,
      format: self.provide_format_info(),
    }
  }

  fn resize(&mut self, renderer: &WGPURenderer, size: (usize, usize)) {
    self.base.resize(renderer, size)
  }

  fn get_size(&self) -> (usize, usize) {
    self.base.size
  }
}
