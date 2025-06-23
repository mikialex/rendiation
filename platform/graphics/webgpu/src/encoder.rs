use reactive::EventSource;

use crate::*;

pub struct GPUCommandEncoder {
  pub(crate) encoder: gpu::CommandEncoder,
  active_pass_target_holder: Option<RenderPassDescription>,
  placeholder_bg: Arc<gpu::BindGroup>,
  deferred_explicit_destroy: CommandBufferDeferExplicitDestroyFlusher,
  pub(crate) on_submit: EventSource<()>,
}

pub struct GPUCommandBuffer {
  pub(crate) inner: gpu::CommandBuffer,
  _deferred_explicit_destroy: CommandBufferDeferExplicitDestroyFlusher,
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
  pub(crate) fn new(encoder: gpu::CommandEncoder, device: &GPUDevice) -> Self {
    Self {
      encoder,
      placeholder_bg: device.inner.placeholder_bg.clone(),
      active_pass_target_holder: Default::default(),
      on_submit: Default::default(),
      deferred_explicit_destroy: device.inner.deferred_explicit_destroy.new_command_buffer(),
    }
  }

  pub fn finish(self) -> GPUCommandBuffer {
    let gpu = self.encoder.finish();
    self.on_submit.emit(&());
    GPUCommandBuffer {
      inner: gpu,
      _deferred_explicit_destroy: self.deferred_explicit_destroy,
    }
  }

  pub fn with_compute_pass_scoped(mut self, f: impl FnOnce(GPUComputePass)) -> Self {
    f(self.begin_compute_pass());
    self
  }

  pub fn compute_pass_scoped(&mut self, f: impl FnOnce(GPUComputePass)) -> &mut Self {
    f(self.begin_compute_pass());
    self
  }

  pub fn begin_compute_pass(&mut self) -> GPUComputePass {
    GPUComputePass {
      pass: self
        .encoder
        .begin_compute_pass(&Default::default())
        .forget_lifetime(),
      placeholder_bg: self.placeholder_bg.clone(),
    }
  }

  pub fn do_u_hear_the_people_sing(&mut self, mut des: RenderPassDescription) {
    des.channels.iter_mut().for_each(|c| {
      c.0 = gpu::Operations {
        load: gpu::LoadOp::Clear(gpu::Color::WHITE),
        store: gpu::StoreOp::Store,
      }
    });
    self.begin_render_pass(des, None);
  }

  pub fn begin_render_pass_with_info(
    &mut self,
    des: RenderPassDescription,
    gpu: GPU,
    enable_time_measuring: bool,
  ) -> FrameRenderPass {
    let buffer_size = des.buffer_size();

    let time_query =
      enable_time_measuring.then(|| TimeQuery::create_query_without_start(&gpu.device));

    let ctx = self.begin_render_pass(des, time_query);

    let pass_info = RenderPassGPUInfoData::new(buffer_size.map(|v| 1.0 / v), buffer_size);
    let pass_info = create_uniform_with_cache(pass_info, &gpu);

    let c = GPURenderPassCtx::new(ctx, gpu);

    FrameRenderPass { ctx: c, pass_info }
  }

  pub fn begin_render_pass(
    &mut self,
    des: RenderPassDescription,
    start_time_query: Option<TimeQuery>,
  ) -> GPURenderPass {
    self.active_pass_target_holder.replace(des);
    let des = self.active_pass_target_holder.as_ref().unwrap();
    let mut size = None;

    let color_attachments: Vec<_> = des
      .channels
      .iter()
      .map(|(ops, view)| {
        size = Some(view.size());
        Some(gpu::RenderPassColorAttachment {
          view: view.as_view(),
          resolve_target: des.resolve_target.as_ref().map(|t| t.as_view()),
          ops: *ops,
          depth_slice: None,
        })
      })
      .collect();

    let depth_stencil_attachment = des.depth_stencil_target.as_ref().map(|(ops, view)| {
      size = Some(view.size());
      gpu::RenderPassDepthStencilAttachment {
        view: view.as_view(),
        depth_ops: (*ops).into(),
        stencil_ops: None,
      }
    });

    let timestamp_writes = start_time_query
      .as_ref()
      .map(|t| RenderPassTimestampWrites {
        query_set: &t.query_set,
        beginning_of_pass_write_index: Some(0),
        end_of_pass_write_index: Some(1),
      });

    let desc = gpu::RenderPassDescriptor {
      label: des.name.as_str().into(),
      color_attachments: color_attachments.as_slice(),
      depth_stencil_attachment,
      timestamp_writes,
      occlusion_query_set: None,
    };

    let formats = RenderTargetFormatsInfo {
      color_formats: des.channels.iter().map(|(_, view)| view.format()).collect(),
      depth_stencil_formats: des
        .depth_stencil_target
        .as_ref()
        .map(|(_, view)| view.format()),
      sample_count: des
        .channels
        .first()
        .map(|c| &c.1)
        .or_else(|| des.depth_stencil_target.as_ref().map(|c| &c.1))
        .map(|c| c.sample_count())
        .unwrap_or(1),
    };

    let pass = self.encoder.begin_render_pass(&desc).forget_lifetime();
    GPURenderPass {
      pass,
      placeholder_bg: self.placeholder_bg.clone(),
      size: size.expect("attachment should exists"),
      formats,
      time_measuring: start_time_query,
    }
  }

  pub fn copy_source_to_texture_2d(
    &mut self,
    device: &GPUDevice,
    source: impl WebGPU2DTextureSource,
    target: &GPU2DTexture,
    origin: (u32, u32),
  ) -> &mut Self {
    let (upload_buffer, size) = source.create_upload_buffer(device);

    self.encoder.copy_buffer_to_texture(
      gpu::TexelCopyBufferInfo {
        buffer: &upload_buffer,
        layout: gpu::TexelCopyBufferLayout {
          offset: 0,
          bytes_per_row: Some(Into::<usize>::into(size.width) as u32),
          rows_per_image: Some(Into::<usize>::into(size.height) as u32),
        },
      },
      gpu::TexelCopyTextureInfo {
        texture: &target.texture,
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
