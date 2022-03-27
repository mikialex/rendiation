use rendiation_algebra::Vec3;
use rendiation_renderable_mesh::vertex::Vertex;
use rendiation_webgpu::*;

use crate::*;

impl MaterialMeshLayoutRequire for PhysicalMaterial {
  type VertexInput = Vec<Vertex>;
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderStruct)]
pub struct PhysicalMaterialUniform {
  pub albedo: Vec3<f32>,
}

impl ShaderHashProvider for PhysicalMaterialGPU {}

pub struct PhysicalMaterialGPU {
  uniform: UniformBufferView<PhysicalMaterialUniform>,
  sampler: GPUSamplerView,
  texture: GPUTexture2dView,
}

impl ShaderPassBuilder for PhysicalMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform, SB::Material);
    ctx.binding.bind(&self.sampler, SB::Material);
    ctx.binding.bind(&self.texture, SB::Material);
  }
}

impl ShaderGraphProvider for PhysicalMaterialGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let uniform = binding.uniform_by(&self.uniform, SB::Material).expand();
      let albedo_tex = binding.uniform_by(&self.texture, SB::Material);
      let sampler = binding.uniform_by(&self.sampler, SB::Material);

      let uv = builder.query::<FragmentUv>()?.get();
      let albedo = albedo_tex.sample(sampler, uv).xyz() * uniform.albedo;

      let result = (albedo, 1.).into();

      builder.set_fragment_out(0, result)
    })
  }
}

impl WebGPUMaterial for PhysicalMaterial {
  type GPU = PhysicalMaterialGPU;

  fn create_gpu(&self, res: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU {
    let uniform = PhysicalMaterialUniform {
      albedo: self.albedo,
    };
    let uniform = UniformBufferResource::create_with_source(uniform, &gpu.device);
    let uniform = uniform.create_view(Default::default());

    let sampler = GPUSampler::create(self.sampler.into(), &gpu.device);
    let sampler = sampler.create_view(Default::default());

    PhysicalMaterialGPU {
      uniform,
      sampler,
      texture: self.texture.check_update_gpu(res, gpu).clone(),
    }
  }
  fn is_keep_mesh_shape(&self) -> bool {
    true
  }
  fn is_transparent(&self) -> bool {
    false
  }
}
