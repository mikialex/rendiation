use std::{cell::RefCell, collections::HashMap, rc::Rc};

use rendiation_algebra::Vec2;
use rendiation_texture::Size;
use rendiation_webgpu::{
  BindGroupDescriptor, BindGroupLayoutProvider, BindableResource, PipelineBuilder, RenderPassInfo,
  UniformBufferDataWithCache, GPU,
};

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
  pub ubo: UniformBufferDataWithCache<RenderPassGPUInfoData>,
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
      buffer_size: buffer_size,
    };

    let g = self.pool.entry(key).or_insert_with(|| {
      let ubo = UniformBufferDataWithCache::create(&gpu.device, info);

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

impl BindGroupLayoutProvider for PassGPUData {
  fn bind_preference() -> usize {
    3
  }

  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::all(),
        ty: UniformBufferDataWithCache::<RenderPassGPUInfoData>::bind_layout(),
        count: None,
      }],
    })
  }

  fn gen_shader_header(group: usize) -> String {
    format!(
      "
      [[group({group}), binding(0)]]
      var<uniform> pass_info: RenderPassGPUInfoData;
    "
    )
  }

  fn register_uniform_struct_declare(builder: &mut PipelineBuilder) {
    builder.declare_uniform_struct::<RenderPassGPUInfoData>();
  }
}
