use std::{
  num::NonZeroU32,
  ops::{Deref, DerefMut},
  rc::Rc,
};

use crate::*;

pub struct GPUCommandEncoder {
  encoder: wgpu::CommandEncoder,
  holder: GPURenderPassDataHolder,
  placeholder_bg: Rc<wgpu::BindGroup>,
}

impl Deref for GPUCommandEncoder {
  type Target = wgpu::CommandEncoder;

  fn deref(&self) -> &Self::Target {
    &self.encoder
  }
}

impl DerefMut for GPUCommandEncoder {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.encoder
  }
}

impl GPUCommandEncoder {
  pub fn new(encoder: wgpu::CommandEncoder, device: &wgpu::Device) -> Self {
    let placeholder_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &PlaceholderBindgroup::layout(device),
      entries: &[],
      label: None,
    });

    Self {
      encoder,
      holder: Default::default(),
      placeholder_bg: Rc::new(placeholder_bg),
    }
  }

  pub fn finish(self) -> wgpu::CommandBuffer {
    self.encoder.finish()
  }

  pub fn begin_render_pass<'a>(
    &'a mut self,
    des: &'a RenderPassDescriptorOwned,
  ) -> GPURenderPass<'a> {
    let color_attachments: Vec<_> = des
      .channels
      .iter()
      .map(|(ops, view, _)| wgpu::RenderPassColorAttachment {
        view,
        resolve_target: des.resolve_target.as_ref().map(|t| t.as_ref()),
        ops: *ops,
      })
      .collect();

    let depth_stencil_attachment =
      des
        .depth_stencil_target
        .as_ref()
        .map(|(ops, view)| wgpu::RenderPassDepthStencilAttachment {
          view,
          depth_ops: (*ops).into(),
          stencil_ops: None,
        });

    let desc = wgpu::RenderPassDescriptor {
      label: des.name.as_str().into(),
      color_attachments: color_attachments.as_slice(),
      depth_stencil_attachment,
    };

    let info = RenderPassInfo {
      buffer_size: des.channels.first().unwrap().2,
      format_info: des.info.clone(),
    };

    let pass = self.encoder.begin_render_pass(&desc);
    GPURenderPass {
      pass,
      holder: &mut self.holder,
      info,
      placeholder_bg: self.placeholder_bg.clone(),
    }
  }

  pub fn copy_source_to_texture_2d(
    &mut self,
    device: &wgpu::Device,
    source: impl WebGPUTexture2dSource,
    target: &WebGPUTexture2d,
    origin: (u32, u32),
  ) -> &mut Self {
    let (upload_buffer, size) = source.create_upload_buffer(device);

    self.encoder.copy_buffer_to_texture(
      wgpu::ImageCopyBuffer {
        buffer: &upload_buffer,
        layout: wgpu::ImageDataLayout {
          offset: 0,
          bytes_per_row: NonZeroU32::new(Into::<usize>::into(size.width) as u32),
          rows_per_image: NonZeroU32::new(Into::<usize>::into(size.height) as u32),
        },
      },
      wgpu::ImageCopyTexture {
        texture: &target.texture,
        mip_level: 0,
        origin: wgpu::Origin3d {
          x: origin.0,
          y: origin.1,
          z: 0,
        },
        aspect: wgpu::TextureAspect::All,
      },
      source.gpu_size(),
    );
    self
  }
}
