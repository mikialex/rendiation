use rendiation_lighting_gpu_system::{LightingComputeComponent, LightingComputeInvocation};
use rendiation_texture_core::TextureSampler;
use rendiation_texture_gpu_base::SamplerConvertExt;

use crate::*;

pub struct IBLLightingComponent {
  pub diffuse: GPUCubeTextureView,
  pub specular: GPUCubeTextureView,
  pub brdf_lut: GPU2DTextureView,
  pub uniform: UniformBufferDataView<IblShaderInfo>,
}

impl ShaderHashProvider for IBLLightingComponent {
  shader_hash_type_id! {}
}

impl LightingComputeComponent for IBLLightingComponent {
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn LightingComputeInvocation> {
    Box::new(IBLLighting {
      diffuse: binding.bind_by(&self.diffuse),
      specular: binding.bind_by(&self.specular),
      brdf_lut: binding.bind_by(&self.brdf_lut),
      sampler: binding.bind_by(&ImmediateGPUSamplerViewBind),
      uniform: binding.bind_by(&self.uniform),
    })
  }

  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.diffuse);
    ctx.binding.bind(&self.specular);
    ctx.binding.bind(&self.brdf_lut);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
    ctx.binding.bind(&self.uniform);
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct IblShaderInfo {
  pub diffuse_illuminance: f32,
  pub specular_illuminance: f32,
  pub roughness_one_level: f32,
}

pub struct IBLLighting {
  pub diffuse: HandleNode<ShaderTextureCube>,
  pub specular: HandleNode<ShaderTextureCube>,
  pub brdf_lut: HandleNode<ShaderTexture2D>,
  pub sampler: HandleNode<ShaderSampler>,
  pub uniform: UniformNode<IblShaderInfo>,
}

impl LightingComputeInvocation for IBLLighting {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let shading = shading
      .as_any()
      .downcast_ref::<ENode<ShaderPhysicalShading>>();
    if shading.is_none() {
      return zeroed_val::<ShaderLightingResult>().expand();
    }

    let ENode::<ShaderPhysicalShading> {
      diffuse: surface_diffuse,
      perceptual_roughness,
      f0,
      emissive,
      ..
    } = shading.cloned().unwrap();

    let uniform = self.uniform.load().expand();

    let diffuse = self
      .diffuse
      .sample_zero_level(self.sampler, geom_ctx.normal);

    let diffuse = diffuse.xyz() * surface_diffuse * uniform.diffuse_illuminance + emissive;

    let lod = perceptual_roughness * uniform.roughness_one_level;
    let specular = self
      .specular
      .build_sample_call(self.sampler, geom_ctx.normal)
      .with_level(lod)
      .sample();

    let n_dot_v = geom_ctx.normal.dot(geom_ctx.view_dir);
    let brdf_lut = self
      .brdf_lut
      .sample_zero_level(self.sampler, (n_dot_v, perceptual_roughness));
    let specular =
      (f0 * brdf_lut.x() + brdf_lut.y().splat()) * specular.xyz() * uniform.specular_illuminance;

    ENode::<ShaderLightingResult> { diffuse, specular }
  }
}
