use std::rc::Rc;

use bytemuck::{Pod, Zeroable};
use rendiation_algebra::*;
use rendiation_webgpu::*;

use crate::{GPUResourceSubCache, ResourceMapper, SceneNode, SceneNodeData, SceneNodeDataImpl};

#[derive(Default)]
pub struct NodeGPU {
  inner: ResourceMapper<TransformGPU, SceneNodeDataImpl>,
}

impl SceneNode {
  pub fn check_update_gpu(&self, resources: &mut GPUResourceSubCache, gpu: &GPU) {
    self.mutate(|node| {
      resources.nodes.check_update_gpu(node, gpu);
    });
  }
}

impl NodeGPU {
  pub fn check_update_gpu(&mut self, node: &mut SceneNodeData, gpu: &GPU) -> &TransformGPU {
    self.get_update_or_insert_with(
      node,
      |node| TransformGPU::new(gpu, &node.world_matrix),
      |node_gpu, node| {
        node_gpu.update(gpu, &node.world_matrix);
      },
    )
  }

  pub fn expect_gpu(&self, node: &SceneNodeData) -> &TransformGPU {
    self.get_unwrap(node)
  }
}

impl std::ops::Deref for NodeGPU {
  type Target = ResourceMapper<TransformGPU, SceneNodeDataImpl>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl std::ops::DerefMut for NodeGPU {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

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
