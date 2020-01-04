use std::collections::HashMap;

pub mod r#const;
pub mod shader_util;
pub mod buffer;
pub mod texture;
pub mod attachment_texture;
pub mod pipeline;
pub mod bindgroup;
pub mod sampler;

pub use pipeline::*;
pub use buffer::*;
pub use texture::*;
pub use attachment_texture::*;
pub use bindgroup::*;
pub use sampler::*;

pub struct WGPURenderer {
  device: wgpu::Device,
  pipelines: HashMap<String, WGPUPipeline>,
  depth: WGPUAttachmentTexture,
  swap_chain: wgpu::SwapChain
}

impl WGPURenderer{
  pub fn new(){

  }

  pub fn resize(&mut self, width:usize, height:usize){
    self.depth.resize(&self.device, width, height)
  }
}
