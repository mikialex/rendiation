use rendiation_algebra::Vec3;
use rendiation_renderable_mesh::vertex::Vertex;
use webgpu::*;

use crate::*;

impl<S: SceneContent> MaterialMeshLayoutRequire for PhysicalMaterial<S> {
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

      builder.set_fragment_out(0, (albedo, 1.))
    })
  }
}

impl<S> WebGPUMaterial for PhysicalMaterial<S>
where
  S: SceneContent,
  S::Texture2D: AsRef<dyn WebGPUTexture2dSource>,
{
  type GPU = PhysicalMaterialGPU;

  fn create_gpu(&self, res: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU {
    let uniform = PhysicalMaterialUniform {
      albedo: self.albedo,
    };
    let uniform = UniformBufferResource::create_with_source(uniform, &gpu.device);
    let uniform = uniform.create_default_view();

    let sampler = GPUSampler::create(self.sampler.into(), &gpu.device);
    let sampler = sampler.create_default_view();

    PhysicalMaterialGPU {
      uniform,
      sampler,
      texture: check_update_gpu_2d::<S>(&self.texture, res, gpu).clone(),
    }
  }
  fn is_keep_mesh_shape(&self) -> bool {
    true
  }
  fn is_transparent(&self) -> bool {
    false
  }
}
