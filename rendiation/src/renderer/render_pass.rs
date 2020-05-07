use crate::{viewport::*, WGPUBindGroup, WGPUBuffer, WGPUPipeline};

pub struct WGPURenderPass<'a> {
  pub gpu_pass: wgpu::RenderPass<'a>,
}

impl<'a> WGPURenderPass<'a> {
  pub fn set_pipeline(&mut self, pipeline: &WGPUPipeline) -> &mut Self {
    self.gpu_pass.set_pipeline(&pipeline.pipeline);
    self
  }

  pub fn set_bindgroup(&mut self, index: usize, bindgroup: &WGPUBindGroup) -> &mut Self {
    self
      .gpu_pass
      .set_bind_group(index as u32, &bindgroup.gpu_bindgroup, &[]);
    self
  }

  pub fn set_index_buffer(&mut self, buffer: &WGPUBuffer) -> &mut Self {
    self.gpu_pass.set_index_buffer(buffer.get_gpu_buffer(), 0);
    self
  }

  pub fn set_vertex_buffers(&mut self, buffers: &[WGPUBuffer]) -> &mut Self {
    let mapped_buffers: Vec<(&wgpu::Buffer, u64)> =
      buffers.iter().map(|b| (b.get_gpu_buffer(), 0)).collect(); // todo use small vec opt
    self.gpu_pass.set_vertex_buffers(0, &mapped_buffers);
    self
  }
}

pub struct WGPURenderPassBuilder<'a> {
  color_attachments: Vec<(&'a wgpu::TextureView, wgpu::Color, bool)>,
  depth_attachments: Option<&'a wgpu::TextureView>,
}

impl<'a> WGPURenderPassBuilder<'a> {
  pub fn output(mut self, target: &'a wgpu::TextureView) -> Self {
    self.color_attachments.push((
      target,
      wgpu::Color {
        r: 0.,
        g: 0.,
        b: 0.,
        a: 0.,
      },
      false,
    ));
    self
  }

  pub fn output_with_clear(
    mut self,
    target: &'a wgpu::TextureView,
    color: (f32, f32, f32, f32),
  ) -> Self {
    self.color_attachments.push((
      target,
      wgpu::Color {
        r: color.0 as f64,
        g: color.1 as f64,
        b: color.2 as f64,
        a: color.3 as f64,
      },
      true,
    ));
    self
  }

  pub fn with_depth(mut self, target: &'a wgpu::TextureView) -> Self {
    self.depth_attachments = Some(target);
    self
  }

  pub fn create(self, encoder: &'a mut wgpu::CommandEncoder) -> WGPURenderPass {
    let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
        attachment: self.color_attachments[0].0,
        resolve_target: None,
        load_op: if self.color_attachments[0].2 {
          wgpu::LoadOp::Clear
        } else {
          wgpu::LoadOp::Load
        },
        store_op: wgpu::StoreOp::Store,
        clear_color: self.color_attachments[0].1,
      }],
      depth_stencil_attachment: self.depth_attachments.map(|depth_view| {
        wgpu::RenderPassDepthStencilAttachmentDescriptor {
          attachment: depth_view,
          depth_load_op: wgpu::LoadOp::Clear,
          depth_store_op: wgpu::StoreOp::Store,
          stencil_load_op: wgpu::LoadOp::Clear,
          stencil_store_op: wgpu::StoreOp::Store,
          clear_depth: 1.0,
          clear_stencil: 0,
        }
      }),
    });

    WGPURenderPass { gpu_pass: pass }
  }
}

impl<'a> WGPURenderPass<'a> {
  pub fn build() -> WGPURenderPassBuilder<'a> {
    WGPURenderPassBuilder {
      color_attachments: Vec::new(),
      depth_attachments: None,
    }
  }

  pub fn use_viewport(&mut self, viewport: &Viewport) -> &mut Self {
    self.gpu_pass.set_viewport(
      viewport.x,
      viewport.y,
      viewport.w,
      viewport.h,
      viewport.min_depth,
      viewport.max_depth,
    );
    self
  }

  pub fn render_object(&mut self) {}
}
