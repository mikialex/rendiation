pub struct WGPURenderPass<'a>{
    swap_chain_output: wgpu::SwapChainOutput<'a>,
    gpu_pass: wgpu::RenderPass<'a>
}

impl<'a> WGPURenderPass<'a>{
    pub fn render_object(&mut self){

    }
}

// trait WGPURenderable{
//     fn render(renderer: WGPURenderer);
// }