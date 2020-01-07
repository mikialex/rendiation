pub struct WGPURenderPass<'a> {
  // swap_chain_output: wgpu::SwapChainOutput<'a>,
  pub gpu_pass: wgpu::RenderPass<'a>,
}

pub struct WGPURenderPassBuilder<'a> {
  color_attachments: Vec<(&'a wgpu::TextureView, Option<wgpu::Color>)>,
  depth_attachments: Option<&'a wgpu::TextureView>,
}

impl<'a> WGPURenderPassBuilder<'a> {
  pub fn output(mut self, target: &'a wgpu::TextureView) -> Self {
    self.color_attachments.push((target, None));
    self
  }

  pub fn output_with_clear(
    mut self,
    target: &'a wgpu::TextureView,
    color: (f32, f32, f32, f32),
  ) -> Self {
    self.color_attachments.push((
      target,
      Some(wgpu::Color {
        r: color.0 as f64,
        g: color.1 as f64,
        b: color.2 as f64,
        a: color.3 as f64,
      }),
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
        load_op: wgpu::LoadOp::Clear,
        store_op: wgpu::StoreOp::Store,
        clear_color: wgpu::Color {
          r: 0.1,
          g: 0.2,
          b: 0.3,
          a: 1.0,
        },
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

  pub fn render_object(&mut self) {}
}

// trait WGPURenderable{
//     fn render(renderer: WGPURenderer);
// }
