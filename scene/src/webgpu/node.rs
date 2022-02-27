use bytemuck::{Pod, Zeroable};
use rendiation_algebra::*;
use rendiation_webgpu::*;
use shadergraph::*;

use crate::*;

#[derive(Default)]
pub struct NodeGPUStore {
  inner: ResourceMapper<TransformGPU, SceneNodeDataImpl>,
}

impl NodeGPUStore {
  pub fn check_update_gpu(&mut self, node: &SceneNode, gpu: &GPU) -> &TransformGPU {
    node.visit(|node| {
      let r = self.get_update_or_insert_with(
        node,
        |node| TransformGPU::new(gpu, &node.world_matrix),
        |node_gpu, node| {
          node_gpu.update(gpu, &node.world_matrix);
        },
      );

      // todo can i workaround this?
      unsafe { std::mem::transmute(r) }
    })
  }
}

impl std::ops::Deref for NodeGPUStore {
  type Target = ResourceMapper<TransformGPU, SceneNodeDataImpl>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl std::ops::DerefMut for NodeGPUStore {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

pub struct TransformGPU {
  pub ubo: UniformBufferData<TransformGPUData>,
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod, Default, PartialEq, ShaderUniform)]
pub struct TransformGPUData {
  pub world_matrix: Mat4<f32>,
}

impl ShaderHashProvider for TransformGPU {}

impl ShaderGraphProvider for TransformGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, binding| {
      let model = binding.uniform_by(&self.ubo, SB::Object).expand();
      let position = builder.query::<GeometryPosition>()?.get_last();
      let position = model.world_matrix * (position, 0.).into();
      builder.register::<WorldVertexPosition>(position.xyz());
      Ok(())
    })
  }
}

impl ShaderBindingProvider for TransformGPU {
  fn setup_binding(&self, builder: &mut BindingBuilder) {
    // builder.setup_uniform(&self.ubo)
  }
}

impl TransformGPU {
  pub fn update(&mut self, gpu: &GPU, matrix: &Mat4<f32>) -> &mut Self {
    self.ubo.world_matrix = *matrix;
    self.ubo.update_with_diff(&gpu.queue);
    self
  }

  pub fn new(gpu: &GPU, matrix: &Mat4<f32>) -> Self {
    let device = &gpu.device;

    let mut ubo: UniformBufferData<TransformGPUData> = UniformBufferData::create_default(device);
    ubo.world_matrix = *matrix;
    ubo.update(&gpu.queue);

    Self { ubo }
  }
}
