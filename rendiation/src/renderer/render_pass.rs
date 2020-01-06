pub struct WGPURenderPass<'a>{
    // swap_chain_output: wgpu::SwapChainOutput<'a>,
    gpu_pass: wgpu::RenderPass<'a>
}

impl<'a> WGPURenderPass<'a>{
    pub fn new(encoder:&'a mut wgpu::CommandEncoder, attachment: &wgpu::TextureView) -> Self{
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
              attachment,
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
            depth_stencil_attachment: None,
          });

        Self{
            gpu_pass: pass
        }
    }

    pub fn render_object(&mut self){

    }
}

// trait WGPURenderable{
//     fn render(renderer: WGPURenderer);
// }