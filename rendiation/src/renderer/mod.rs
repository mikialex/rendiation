use std::collections::HashMap;

pub mod r#const;
pub mod shader_util;
pub mod buffer;
pub mod texture;
pub mod pipeline;
pub mod bindgroup;
pub mod sampler;

pub use pipeline::*;
pub use buffer::*;
pub use texture::*;
pub use bindgroup::*;
pub use sampler::*;

pub struct WGPURenderer {
  device: wgpu::Device,
  pipelines: HashMap<String, WGPUPipeline>,
}
