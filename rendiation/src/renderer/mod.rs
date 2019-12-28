use std::collections::HashMap;

pub mod r#const;
pub mod shader_util;
pub mod buffer;
pub mod texture;
pub mod pipeline;

use pipeline::*;

pub struct WGPURenderer {
  device: wgpu::Device,
  pipelines: HashMap<String, WGPUPipeline>,
}
