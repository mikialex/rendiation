use rendiation_algebra::Vec4;
use rendiation_renderable_mesh::vertex::Vertex;
use rendiation_webgpu::*;

use crate::*;

impl MaterialMeshLayoutRequire for FlatMaterial {
  type VertexInput = Vec<Vertex>;
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderStruct)]
pub struct FlatMaterialUniform {
  pub color: Vec4<f32>,
}

pub struct FlatMaterialGPU {
  uniform: UniformBufferView<FlatMaterialUniform>,
}

impl ShaderHashProvider for FlatMaterialGPU {}

impl ShaderGraphProvider for FlatMaterialGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let uniform = binding.uniform_by(&self.uniform, SB::Material).expand();

      builder.set_fragment_out(0, uniform.color)
    })
  }
}

impl ShaderPassBuilder for FlatMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform, SB::Material);
  }
}

impl WebGPUMaterial for FlatMaterial {
  type GPU = FlatMaterialGPU;

  fn create_gpu(&self, _: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU {
    let uniform = FlatMaterialUniform { color: self.color };
    let uniform = UniformBufferResource::create_with_source(uniform, &gpu.device);
    let uniform = uniform.create_default_view();

    FlatMaterialGPU { uniform }
  }

  fn is_keep_mesh_shape(&self) -> bool {
    true
  }

  fn is_transparent(&self) -> bool {
    false
  }
}
