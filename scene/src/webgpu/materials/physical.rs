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

pub struct PhysicalMaterialGPU {
  uniform: UniformBuffer<Vec3<f32>>,
  sampler: GPUSampler,
  texture: GPUTexture,
}

impl ShaderBindingProvider for PhysicalMaterialGPU {
  fn setup_binding(&self, builder: &mut BindingBuilder) {
    builder.setup_uniform(&self.uniform);
    builder.setup_uniform(&self.sampler);
    builder.setup_uniform(&self.texture);
  }
}

impl ShaderGraphProvider for PhysicalMaterialGPU {
  fn build_fragment(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let uniform = builder
      .register_uniform_by(&self.uniform, SB::Material)
      .expand();

    let result = (uniform.albedo, 1.).into();

    builder.set_fragment_out(0, result);
    Ok(())
  }
}

impl WebGPUMaterial for PhysicalMaterial {
  type GPU = PhysicalMaterialGPU;

  fn create_gpu(&self, res: &mut GPUResourceSubCache) -> Self::GPU {
    PhysicalMaterialGPU {
      uniform: res.uniforms.get(self.albedo),
      sampler: res.samplers.get(self.sampler),
      texture: res.texture_2ds.get(self.texture),
    }
  }
  fn is_keep_mesh_shape(&self) -> bool {
    true
  }
  fn is_transparent(&self) -> bool {
    false
  }
}
