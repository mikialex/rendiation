use std::collections::HashMap;

pub mod r#const;
pub mod shader_util;
pub mod buffer;
pub mod texture;
pub mod pipeline;

pub use pipeline::*;
pub use buffer::*;
pub use texture::*;

pub struct WGPURenderer {
  device: wgpu::Device,
  pipelines: HashMap<String, WGPUPipeline>,
}
