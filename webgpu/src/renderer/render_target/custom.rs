use rendiation_ral::{RenderTargetFormatsInfo, TargetStates};

use crate::{
  RenderTargetAble, TargetInfoProvider, WGPURenderPassBuilder, WGPURenderer, WGPUTexture,
};

pub struct RenderTarget {
  attachments: Vec<WGPUTexture>,
  depth: Option<WGPUTexture>,
}

impl RenderTarget {
  pub fn new(attachments: Vec<WGPUTexture>, depth: Option<WGPUTexture>) -> Self {
    Self { attachments, depth }
  }

  pub fn from_one_texture(attachment: WGPUTexture) -> Self {
    RenderTarget::new(vec![attachment], None)
  }

  pub fn from_one_texture_and_depth(attachment: WGPUTexture, depth: WGPUTexture) -> Self {
    RenderTarget::new(vec![attachment], Some(depth))
  }

  pub fn get_nth_color_attachment(&self, n: usize) -> &WGPUTexture {
    &self.attachments[n]
  }

  pub fn get_first_color_attachment(&self) -> &WGPUTexture {
    self.get_nth_color_attachment(0)
  }

  pub fn dissemble(self) -> (Vec<WGPUTexture>, Option<WGPUTexture>) {
    (self.attachments, self.depth)
  }
}

impl TargetInfoProvider for RenderTarget {
  fn create_target_states(&self) -> TargetStates {
    let color_states = self
      .attachments
      .iter()
      .map(|t| wgpu::ColorStateDescriptor {
        format: *t.format(),
        color_blend: wgpu::BlendDescriptor::REPLACE,
        alpha_blend: wgpu::BlendDescriptor::REPLACE,
        write_mask: wgpu::ColorWrite::ALL,
      })
      .collect();

    let depth_state = self
      .depth
      .as_ref()
      .map(|d| wgpu::DepthStencilStateDescriptor {
        format: *d.format(),
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil: wgpu::StencilStateDescriptor::default(),
      });

    TargetStates {
      color_states,
      depth_state,
    }
  }

  fn provide_format_info(&self) -> RenderTargetFormatsInfo {
    let color = self.attachments.iter().map(|t| *t.format()).collect();
    let depth = self.depth.as_ref().map(|d| *d.format());
    RenderTargetFormatsInfo { color, depth }
  }
}

impl RenderTargetAble for RenderTarget {
  fn create_render_pass_builder(&self) -> WGPURenderPassBuilder {
    let attachments = self
      .attachments
      .iter()
      .map(|att| wgpu::RenderPassColorAttachmentDescriptor {
        attachment: att.view(),
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Load,
          store: true,
        },
      })
      .collect();
    let depth = self
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
    self
      .attachments
      .iter_mut()
      .for_each(|color| color.resize(renderer, size));
    self
      .depth
      .as_mut()
      .map(|depth| depth.resize(renderer, size));
  }

  fn get_size(&self) -> (usize, usize) {
    self.attachments.iter().next().unwrap().size().to_tuple()
  }
}
