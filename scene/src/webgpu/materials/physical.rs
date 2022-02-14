use std::rc::Rc;

use rendiation_algebra::Vec3;
use rendiation_renderable_mesh::vertex::Vertex;
use rendiation_webgpu::*;

use crate::*;

impl MaterialMeshLayoutRequire for PhysicalMaterial {
  type VertexInput = Vec<Vertex>;
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderUniform)]
pub struct PhysicalMaterialUniform {
  pub albedo: Vec3<f32>,
}

impl SemanticShaderUniform for PhysicalMaterialUniform {
  const TYPE: SemanticBinding = SemanticBinding::Material;
  type Node = Self;
}

pub struct PhysicalMaterialGPU {
  _uniform: UniformBuffer<Vec3<f32>>,
  // sampler: GPUSampler,
  // texture: GPUTexture,
}

impl ShaderGraphProvider for PhysicalMaterialGPU {
  fn build_fragment(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let uniform = builder
      .register_uniform::<PhysicalMaterialUniform>()
      .expand();

    let result = (uniform.albedo, 1.).into();

    builder.set_fragment_out(0, result);
    Ok(())
  }
}

impl MaterialCPUResource for PhysicalMaterial {
  type GPU = PhysicalMaterialGPU;

  fn create(&self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtx) -> Self::GPU {
    let _uniform = UniformBuffer::create(&gpu.device, self.albedo);

    let sampler = ctx.map_sampler(self.sampler, &gpu.device);
    let bindgroup = MaterialBindGroupBuilder::new(gpu, ctx.resources, bgw.clone())
      .push(_uniform.as_bindable())
      .push_texture(&self.texture)
      .push(sampler.as_bindable())
      .build(&bindgroup_layout);

    PhysicalMaterialGPU { _uniform }
  }
  fn is_keep_mesh_shape(&self) -> bool {
    true
  }
  fn is_transparent(&self) -> bool {
    false
  }
}
