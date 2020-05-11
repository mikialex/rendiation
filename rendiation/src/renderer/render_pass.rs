use crate::{viewport::*, WGPUBindGroup, WGPUBuffer, WGPUPipeline};
use rendiation_math::Vec3;
use std::ops::Range;

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

  pub fn draw_indexed(&mut self, index_range: Range<u32>) {
    self.gpu_pass.draw_indexed(index_range, 0, 0..1);
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
}

pub struct WGPURenderPassBuilder<'a> {
  pub attachments: Vec<wgpu::RenderPassColorAttachmentDescriptor<'a>>,
  pub depth: Option<wgpu::RenderPassDepthStencilAttachmentDescriptor<&'a wgpu::TextureView>>,
}

pub struct RenderPassColorAttachmentDescriptorModifier<'a, 'b> {
  attachment: &'a mut wgpu::RenderPassColorAttachmentDescriptor<'b>,
}

impl<'a, 'b> RenderPassColorAttachmentDescriptorModifier<'a, 'b> {
  pub fn load_with_clear(&mut self, clear_color: Vec3<f32>, alpha: f32) -> &mut Self {
    self.attachment.load_op = wgpu::LoadOp::Clear;
    self.attachment.clear_color = wgpu::Color {
      r: clear_color.x as f64,
      g: clear_color.y as f64,
      b: clear_color.z as f64,
      a: alpha as f64,
    };
    self
  }

  pub fn ok(&mut self) {}
}

pub struct RenderPassDepthStencilAttachmentDescriptorModifier<'a, 'b> {
  depth: &'a mut wgpu::RenderPassDepthStencilAttachmentDescriptor<&'b wgpu::TextureView>,
}

impl<'a, 'b> RenderPassDepthStencilAttachmentDescriptorModifier<'a, 'b> {
  pub fn load_with_clear(&mut self, depth: f32) -> &mut Self {
    self.depth.depth_load_op = wgpu::LoadOp::Clear;
    self.depth.clear_depth = depth;
    self
  }
  pub fn ok(&mut self) {}
}

impl<'a> WGPURenderPassBuilder<'a> {
  pub fn nth_color(
    &mut self,
    i: usize,
    visitor: impl Fn(&mut RenderPassColorAttachmentDescriptorModifier),
  ) -> &mut Self {
    let mut modifier = RenderPassColorAttachmentDescriptorModifier {
      attachment: &mut self.attachments[i],
    };
    visitor(&mut modifier);
    self
  }

  pub fn first_color(
    mut self,
    visitor: impl Fn(&mut RenderPassColorAttachmentDescriptorModifier),
  ) -> Self {
    &mut self.nth_color(0, visitor);
    self
  }

  pub fn depth(
    mut self,
    visitor: impl Fn(&mut RenderPassDepthStencilAttachmentDescriptorModifier),
  ) -> Self {
    if let Some(depth) = &mut self.depth {
      let mut modifier = RenderPassDepthStencilAttachmentDescriptorModifier { depth };
      visitor(&mut modifier);
    } else {
      // do we need panic here?
    }
    self
  }

  pub fn create(self, encoder: &'a mut wgpu::CommandEncoder) -> WGPURenderPass {
    let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      color_attachments: &self.attachments,
      depth_stencil_attachment: self.depth,
    });

    WGPURenderPass { gpu_pass: pass }
  }
}
