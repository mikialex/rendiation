use crate::*;

pub struct GPUCommandEncoder {
  encoder: gpu::CommandEncoder,
  holder: GPURenderPassDataHolder,
  active_pass_target_holder: Option<RenderPassDescriptorOwned>,
  placeholder_bg: Rc<gpu::BindGroup>,
}

impl Deref for GPUCommandEncoder {
  type Target = gpu::CommandEncoder;

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
  pub fn new(encoder: gpu::CommandEncoder, device: &GPUDevice) -> Self {
    Self {
      encoder,
      holder: Default::default(),
      placeholder_bg: device.inner.placeholder_bg.clone(),
      active_pass_target_holder: Default::default(),
    }
  }

  pub fn finish(self) -> gpu::CommandBuffer {
    self.encoder.finish()
  }

  pub fn begin_render_pass(&mut self, des: RenderPassDescriptorOwned) -> GPURenderPass {
    self.active_pass_target_holder.replace(des);
    let des = self.active_pass_target_holder.as_ref().unwrap();
    // todo should we do some check here?
    let mut size = Size::from_u32_pair_min_one((100, 100));

    let color_attachments: Vec<_> = des
      .channels
      .iter()
      .map(|(ops, view)| {
        size = view.size();
        Some(gpu::RenderPassColorAttachment {
          view: view.as_view(),
          resolve_target: des.resolve_target.as_ref().map(|t| t.as_view()),
          ops: *ops,
        })
      })
      .collect();

    let depth_stencil_attachment =
      des
        .depth_stencil_target
        .as_ref()
        .map(|(ops, view)| gpu::RenderPassDepthStencilAttachment {
          view: view.as_view(),
          depth_ops: (*ops).into(),
          stencil_ops: None,
        });

    let desc = gpu::RenderPassDescriptor {
      label: des.name.as_str().into(),
      color_attachments: color_attachments.as_slice(),
      depth_stencil_attachment,
    };

    let formats = RenderTargetFormatsInfo {
      color_formats: des.channels.iter().map(|(_, view)| view.format()).collect(),
      depth_stencil_formats: des
        .depth_stencil_target
        .as_ref()
        .map(|(_, view)| view.format()),
      sample_count: des.channels.first().unwrap().1.sample_count(),
    };

    let pass = self.encoder.begin_render_pass(&desc);
    GPURenderPass {
      pass,
      holder: &mut self.holder,
      placeholder_bg: self.placeholder_bg.clone(),
      size,
      formats,
      bundle_encoder: None,
      bundles: Default::default(),
    }
  }

  pub fn copy_source_to_texture_2d(
    &mut self,
    device: &GPUDevice,
    source: impl WebGPUTexture2dSource,
    target: &GPUTexture2d,
    origin: (u32, u32),
  ) -> &mut Self {
    let (upload_buffer, size) = source.create_upload_buffer(device);

    self.encoder.copy_buffer_to_texture(
      gpu::ImageCopyBuffer {
        buffer: &upload_buffer,
        layout: gpu::ImageDataLayout {
          offset: 0,
          bytes_per_row: NonZeroU32::new(Into::<usize>::into(size.width) as u32),
          rows_per_image: NonZeroU32::new(Into::<usize>::into(size.height) as u32),
        },
      },
      gpu::ImageCopyTexture {
        texture: &target.0,
        mip_level: 0,
        origin: gpu::Origin3d {
          x: origin.0,
          y: origin.1,
          z: 0,
        },
        aspect: gpu::TextureAspect::All,
      },
      source.gpu_size(),
    );
    self
  }
}
