use std::{cell::RefCell, collections::HashMap, rc::Rc};

use rendiation_algebra::Vec2;
use rendiation_texture::Size;
use rendiation_webgpu::{BindGroupDescriptor, RenderPassInfo, UniformBufferData, GPU};

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
  pub pool: HashMap<u64, PassGPUData>,
}

pub struct PassGPUData {
  pub ubo: UniformBufferData<RenderPassGPUInfoData>,
  pub bindgroup: Rc<wgpu::BindGroup>,
}

impl PassGPUDataCache {
  pub fn get_updated_pass_gpu_info(
    &mut self,
    key: u64,
    pass_info: &RenderPassInfo,
    gpu: &GPU,
  ) -> &PassGPUData {
    let buffer_size = pass_info.buffer_size.into_usize();
    let buffer_size: Vec2<f32> = (buffer_size.0 as f32, buffer_size.1 as f32).into();
    let info = RenderPassGPUInfoData {
      texel_size: buffer_size.map(|v| 1. / v),
      buffer_size,
    };

    let g = self.pool.entry(key).or_insert_with(|| {
      let ubo = UniformBufferData::create(&gpu.device, info);

      let bindgroup = gpu.device.create_bind_group(&BindGroupDescriptor {
        layout: &PassGPUData::layout(&gpu.device),
        entries: &[wgpu::BindGroupEntry {
          binding: 0,
          resource: ubo.as_bindable(),
        }],
        label: None,
      });

      PassGPUData {
        ubo,
        bindgroup: Rc::new(bindgroup),
      }
    });

    *g.ubo = info;
    g.ubo.update(&gpu.queue);

    g
  }
}
