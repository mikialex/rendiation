use rendiation_lighting_gpu_system::{LightingComputeComponent, LightingComputeInvocation};
use rendiation_texture_core::TextureSampler;
use rendiation_texture_gpu_base::SamplerConvertExt;

use crate::*;

pub struct IBLLightingComponent {
  pub prefiltered: PreFilterMapGenerationResult,
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
    _scene_id: Node<u32>, // the resource is select at host side
  ) -> Box<dyn LightingComputeInvocation> {
    Box::new(IBLLighting {
      diffuse: binding.bind_by(&self.prefiltered.diffuse),
      specular: binding.bind_by(&self.prefiltered.specular),
      brdf_lut: binding.bind_by(&self.brdf_lut),
      sampler: binding.bind_by(&ImmediateGPUSamplerViewBind),
      uniform: binding.bind_by(&self.uniform),
    })
  }

  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.prefiltered.diffuse);
    ctx.binding.bind(&self.prefiltered.specular);
    ctx.binding.bind(&self.brdf_lut);
    ctx.bind_immediate_sampler(&TextureSampler::default().with_double_linear().into_gpu());
    ctx.binding.bind(&self.uniform);
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct IblShaderInfo {
  pub diffuse_illuminance: f32,
  pub specular_illuminance: f32,
}

pub struct IBLLighting {
  pub diffuse: BindingNode<ShaderTextureCube>,
  pub specular: BindingNode<ShaderTextureCube>,
  pub brdf_lut: BindingNode<ShaderTexture2D>,
  pub sampler: BindingNode<ShaderSampler>,
  pub uniform: ShaderReadonlyPtrOf<IblShaderInfo>,
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
      albedo,
      linear_roughness,
      f0,
      emissive,
      ..
    } = shading.cloned().unwrap();

    let uniform = self.uniform.load().expand();

    let diffuse = self
      .diffuse
      .sample_zero_level(self.sampler, geom_ctx.normal);

    let diffuse = diffuse.xyz() * uniform.diffuse_illuminance * albedo + emissive;

    let lod = linear_roughness * (self.specular.texture_number_levels() - val(1)).into_f32();
    let specular = self
      .specular
      .build_sample_call(self.sampler, geom_ctx.normal)
      .with_level(lod)
      .sample();

    let n_dot_v = geom_ctx.normal.dot(geom_ctx.view_dir);
    let brdf_lut = self
      .brdf_lut
      .sample_zero_level(self.sampler, (n_dot_v, linear_roughness));
    let specular =
      (f0 * brdf_lut.x() + brdf_lut.y().splat()) * specular.xyz() * uniform.specular_illuminance;

    ENode::<ShaderLightingResult> {
      diffuse,
      specular_and_emissive: specular,
    }
  }
}
