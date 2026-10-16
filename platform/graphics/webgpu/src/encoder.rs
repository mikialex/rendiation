use crate::*;

pub struct GPUCommandEncoder {
  pub(crate) encoder: gpu::CommandEncoder,
  placeholder_bg: gpu::BindGroup,
  deferred_explicit_destroy: CommandBufferDeferExplicitDestroyFlusher,
  pub(crate) on_submit: EventSource<()>,
}

pub struct GPUCommandBuffer {
  pub(crate) inner: gpu::CommandBuffer,
  pub(crate) on_submit: EventSource<()>,
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
      on_submit: Default::default(),
      deferred_explicit_destroy: device.inner.deferred_explicit_destroy.new_command_buffer(),
    }
  }

  pub fn finish(self) -> GPUCommandBuffer {
    let gpu = self.encoder.finish();
    GPUCommandBuffer {
      inner: gpu,
      on_submit: self.on_submit,
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

  /// the attachments are kept alive by the underlayer render pass, so the description is not
  /// required to outlive the returned pass.
  pub fn begin_render_pass(
    &mut self,
    des: &RenderPassDescription,
    timestamp_writes: Option<gpu::RenderPassTimestampWrites>,
  ) -> GPURenderPass {
    let mut size = None;

    let color_attachments: Vec<_> = des
      .channels
      .iter()
      .map(|(ops, view, resolve_target)| {
        size = Some(view.size());
        Some(gpu::RenderPassColorAttachment {
          view: view.as_render_view(),
          resolve_target: resolve_target.as_ref().map(|t| t.as_render_view()),
          ops: *ops,
          depth_slice: None,
        })
      })
      .collect();

    let depth_stencil_attachment =
      des
        .depth_stencil_target
        .as_ref()
        .map(|(depth_ops, stencil_ops, view)| {
          size = Some(view.size());
          let depth_ops = depth_ops.filter(|_| view.format().has_depth_aspect());
          let stencil_ops = stencil_ops.filter(|_| view.format().has_stencil_aspect());

          gpu::RenderPassDepthStencilAttachment {
            view: view.as_render_view(),
            depth_ops,
            stencil_ops,
          }
        });

    // we do checking here because wgpu validation check is deferred and hard to debug related issue.
    #[cfg(debug_assertions)]
    {
      let iter = des.channels.iter().map(|v| v.1.sample_count());
      let mut _iter = iter.clone();
      let all_equal = match _iter.next() {
        None => true, // Empty iterators are technically all equal
        Some(first) => _iter.all(|x| x == first),
      };

      assert!(all_equal);

      let depth_sample_count = des
        .depth_stencil_target
        .as_ref()
        .map(|v| v.2.sample_count());
      if let (Some(a), Some(b)) = (iter.clone().next(), depth_sample_count) {
        assert_eq!(a, b);
      }
    }

    let desc = gpu::RenderPassDescriptor {
      label: des.name.as_str().into(),
      color_attachments: color_attachments.as_slice(),
      depth_stencil_attachment,
      timestamp_writes,
      occlusion_query_set: None,
      multiview_mask: None,
    };

    let formats = RenderTargetFormatsInfo {
      color_formats: des
        .channels
        .iter()
        .map(|(_, view, _)| view.format())
        .collect(),
      depth_stencil_formats: des
        .depth_stencil_target
        .as_ref()
        .map(|(_, _, view)| view.format()),
      sample_count: des
        .channels
        .first()
        .map(|c| &c.1)
        .or_else(|| des.depth_stencil_target.as_ref().map(|c| &c.2))
        .map(|c| c.sample_count())
        .unwrap_or(1),
    };

    let pass = self.encoder.begin_render_pass(&desc).forget_lifetime();
    GPURenderPass {
      pass,
      placeholder_bg: self.placeholder_bg.clone(),
      size: size.expect("attachment should exists"),
      formats,
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
