use std::rc::Rc;

use bytemuck::{Pod, Zeroable};
use rendiation_algebra::*;
use rendiation_webgpu::*;

pub struct TransformGPU {
  pub ubo: UniformBufferDataWithCache<TransformGPUData>,
  pub bindgroup: Rc<wgpu::BindGroup>,
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod, Default, PartialEq)]
pub struct TransformGPUData {
  pub world_matrix: Mat4<f32>,
}

impl ShaderUniformBlock for TransformGPUData {
  fn shader_struct() -> &'static str {
    "
        struct ModelTransform {
          matrix: mat4x4<f32>;
        };
      "
  }
}

impl BindGroupLayoutProvider for TransformGPU {
  fn bind_preference() -> usize {
    0
  }
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: "ModelTransformBindgroup".into(),
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: wgpu::BufferSize::new(64),
        },
        count: None,
      }],
    })
  }

  fn gen_shader_header(group: usize) -> String {
    format!(
      "
        [[group({group}), binding(0)]]
        var<uniform> model: ModelTransform;
      
      "
    )
  }

  fn register_uniform_struct_declare(builder: &mut PipelineBuilder) {
    builder.declare_uniform_struct::<TransformGPUData>();
  }
}

impl TransformGPU {
  pub fn update(&mut self, gpu: &GPU, matrix: &Mat4<f32>) -> &mut Self {
    self.ubo.world_matrix = *matrix;
    self.ubo.update(&gpu.queue);
    self
  }

  pub fn new(gpu: &GPU, matrix: &Mat4<f32>) -> Self {
    let device = &gpu.device;

    let mut ubo: UniformBufferDataWithCache<TransformGPUData> =
      UniformBufferDataWithCache::create_default(device);
    ubo.world_matrix = *matrix;
    ubo.update(&gpu.queue);

    let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &Self::layout(device),
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: ubo.as_bindable(),
      }],
      label: None,
    });

    let bindgroup = Rc::new(bindgroup);

    Self { ubo, bindgroup }
  }
}
