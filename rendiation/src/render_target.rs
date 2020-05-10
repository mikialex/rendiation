use crate::{texture_format::TextureFormat, WGPUTexture};

struct RenderTarget {
  attachments: Vec<WGPUTexture>,
  depth: Option<WGPUTexture>,
}

impl RenderTarget {
  // todo with builder
  pub fn new() {}

  pub fn get_nth_color_attachment(&self, n: usize) -> &WGPUTexture {
    &self.attachments[n]
  }

  pub fn get_first_color_attachment(&self) -> &WGPUTexture {
    &self.attachments[0]
  }

  pub fn create_target_states_configurer(&self) -> TargetStates {
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

#[derive(Clone)]
pub struct TargetStates {
  pub color_states: Vec<wgpu::ColorStateDescriptor>,
  pub depth_state: Option<wgpu::DepthStencilStateDescriptor>,
}

impl Default for TargetStates {
  fn default() -> Self {
    Self {
      color_states: vec![wgpu::ColorStateDescriptor {
        format: TextureFormat::Rgba8UnormSrgb.get_wgpu_format(),
        color_blend: wgpu::BlendDescriptor::REPLACE,
        alpha_blend: wgpu::BlendDescriptor::REPLACE,
        write_mask: wgpu::ColorWrite::ALL,
      }],
      depth_state: None,
    }
  }
}
