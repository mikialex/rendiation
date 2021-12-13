use std::{cell::RefCell, collections::HashMap, rc::Rc};

use rendiation_texture::Size;
use rendiation_webgpu::UniformBufferData;

use crate::RenderPassGPUInfoData;

#[derive(Default)]
pub struct ResourcePoolImpl {
  pub attachments: HashMap<(Size, wgpu::TextureFormat, u32), Vec<wgpu::Texture>>,
}

#[derive(Clone, Default)]
pub struct ResourcePool {
  pub inner: Rc<RefCell<ResourcePoolImpl>>,
}

#[derive(Default)]
pub struct PassGPUDataCache {
  pub ubo: HashMap<u64, UniformBufferData<RenderPassGPUInfoData>>,
}
