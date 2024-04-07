use std::any::Any;

use rendiation_lighting_transport::{
  AlphaChannel, AlphaCutChannel, EmissiveChannel, LightableSurfaceShadingDyn, MetallicChannel,
  PhysicalShading, RoughnessChannel, ShadingSelection,
};
use rendiation_shader_library::normal_mapping::apply_normal_mapping_conditional;
use rendiation_texture_gpu_system::GPUTextureBindingSystem;

use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct PhysicalMetallicRoughnessMaterialUniform {
  pub base_color: Vec3<f32>,
  pub emissive: Vec3<f32>,
  pub roughness: f32,
  pub metallic: f32,
  pub reflectance: f32,
  pub normal_mapping_scale: f32,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}
type Uniform = PhysicalMetallicRoughnessMaterialUniform;

pub type PbrMRMaterialUniforms = UniformUpdateContainer<PbrMRMaterialEntity, Uniform>;
pub fn pbr_mr_material_uniforms(cx: &GPUResourceCtx) -> PbrMRMaterialUniforms {
  let base_color = global_watch()
    .watch_typed_key::<PbrMRMaterialBaseColorComponent>()
    .into_uniform_collection_update(offset_of!(Uniform, base_color), cx);

  let emissive = global_watch()
    .watch_typed_key::<PbrMRMaterialEmissiveComponent>()
    .into_uniform_collection_update(offset_of!(Uniform, emissive), cx);

  let normal_mapping_scale = global_watch()
    .watch_typed_key::<NormalScaleOf<PbrMRMaterialNormalInfo>>()
    .into_uniform_collection_update(offset_of!(Uniform, normal_mapping_scale), cx);

  let roughness = global_watch()
    .watch_typed_key::<PbrMRMaterialRoughnessComponent>()
    .into_uniform_collection_update(offset_of!(Uniform, roughness), cx);

  let metallic = global_watch()
    .watch_typed_key::<PbrMRMaterialMetallicComponent>()
    .into_uniform_collection_update(offset_of!(Uniform, metallic), cx);

  let alpha = global_watch()
    .watch_typed_key::<PbrMRMaterialAlphaComponent>()
    .into_uniform_collection_update(offset_of!(Uniform, alpha), cx);

  PbrMRMaterialUniforms::default()
    .with_source(base_color)
    .with_source(emissive)
    .with_source(normal_mapping_scale)
    .with_source(roughness)
    .with_source(metallic)
    .with_source(alpha)
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct PhysicalMetallicRoughnessMaterialTextureHandlesUniform {
  pub base_color_texture: TextureSamplerHandlePair,
  pub emissive_texture: TextureSamplerHandlePair,
  pub metallic_roughness_texture: TextureSamplerHandlePair,
  pub normal_texture: TextureSamplerHandlePair,
}
type TexUniform = PhysicalMetallicRoughnessMaterialTextureHandlesUniform;

pub type PbrMRMaterialTexUniforms = UniformUpdateContainer<PbrMRMaterialEntity, TexUniform>;
pub fn pbr_mr_material_tex_uniforms(cx: &GPUResourceCtx) -> PbrMRMaterialTexUniforms {
  let tex_offset = offset_of!(TextureSamplerHandlePair, texture_handle);
  let sam_offset = offset_of!(TextureSamplerHandlePair, sampler_handle);

  let base_color_texture = global_watch()
    .watch_typed_key::<SceneTexture2dRefOf<PbrMRMaterialBaseColorTex>>()
    .collective_map(|id| id.unwrap_or(0))
    .into_uniform_collection_update(offset_of!(TexUniform, base_color_texture) + tex_offset, cx);

  let base_color_sampler = global_watch()
    .watch_typed_key::<SceneSamplerRefOf<PbrMRMaterialBaseColorTex>>()
    .collective_map(|id| id.unwrap_or(0))
    .into_uniform_collection_update(offset_of!(TexUniform, base_color_texture) + sam_offset, cx);

  PbrMRMaterialTexUniforms::default()
    .with_source(base_color_texture)
    .with_source(base_color_sampler)
}

pub fn pbr_mr_material_pipeline_hash(
) -> impl ReactiveCollection<AllocIdx<PbrMRMaterialEntity>, AlphaMode> {
  global_watch().watch_typed_key::<PbrMRMaterialAlphaModeComponent>()
}

pub struct PhysicalMetallicRoughnessMaterialGPU<'a> {
  uniform: &'a UniformBufferDataView<PhysicalMetallicRoughnessMaterialUniform>,
  alpha_mode: AlphaMode,
  // these idx is only useful in per object binding mode
  base_color_tex_sampler: (u32, u32),
  mr_tex_sampler: (u32, u32),
  emissive_tex_sampler: (u32, u32),
  normal_tex_sampler: (u32, u32),
  // no matter if we using indirect texture binding, this uniform is required for checking the
  // texture if is exist in shader
  texture_uniforms:
    &'a UniformBufferDataView<PhysicalMetallicRoughnessMaterialTextureHandlesUniform>,
  binding_sys: &'a GPUTextureBindingSystem,
}

impl<'a> ShaderHashProvider for PhysicalMetallicRoughnessMaterialGPU<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
}

impl<'a> ShaderPassBuilder for PhysicalMetallicRoughnessMaterialGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
    ctx.binding.bind(self.texture_uniforms);
    setup_tex(ctx, self.binding_sys, self.base_color_tex_sampler);
    setup_tex(ctx, self.binding_sys, self.mr_tex_sampler);
    setup_tex(ctx, self.binding_sys, self.emissive_tex_sampler);
    setup_tex(ctx, self.binding_sys, self.normal_tex_sampler);
  }
}

impl<'a> GraphicsShaderProvider for PhysicalMetallicRoughnessMaterialGPU<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.context.insert(
      ShadingSelection.type_id(),
      Box::new(&PhysicalShading as &dyn LightableSurfaceShadingDyn),
    );

    builder.fragment(|builder, binding| {
      let uniform = binding.bind_by(&self.uniform).load().expand();
      let tex_uniform = binding.bind_by(&self.texture_uniforms).load().expand();

      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();

      let mut alpha = uniform.alpha;
      let mut base_color = uniform.base_color;

      let base_color_tex = bind_and_sample(
        self.binding_sys,
        binding,
        builder.registry(),
        self.base_color_tex_sampler,
        tex_uniform.base_color_texture,
        uv,
        val(Vec4::one()),
      );
      alpha *= base_color_tex.w();
      base_color *= base_color_tex.xyz();

      let mut metallic = uniform.metallic;
      let mut roughness = uniform.roughness;

      let metallic_roughness_tex = bind_and_sample(
        self.binding_sys,
        binding,
        builder.registry(),
        self.mr_tex_sampler,
        tex_uniform.metallic_roughness_texture,
        uv,
        val(Vec4::one()),
      );

      metallic *= metallic_roughness_tex.x();
      roughness *= metallic_roughness_tex.y();

      let mut emissive = uniform.emissive;
      emissive *= bind_and_sample(
        self.binding_sys,
        binding,
        builder.registry(),
        self.emissive_tex_sampler,
        tex_uniform.emissive_texture,
        uv,
        val(Vec4::one()),
      )
      .xyz();

      let (normal_sample, enabled) = bind_and_sample_enabled(
        self.binding_sys,
        binding,
        builder.registry(),
        self.normal_tex_sampler,
        tex_uniform.normal_texture,
        uv,
      );

      apply_normal_mapping_conditional(
        builder,
        normal_sample.xyz(),
        uv,
        uniform.normal_mapping_scale,
        enabled,
      );

      match self.alpha_mode {
        AlphaMode::Opaque => {}
        AlphaMode::Mask => {
          let alpha = alpha.less_than(uniform.alpha_cutoff).select(val(0.), alpha);
          builder.register::<AlphaChannel>(alpha);
          builder.register::<AlphaCutChannel>(uniform.alpha_cutoff);
        }
        AlphaMode::Blend => {
          builder.register::<AlphaChannel>(alpha);
          builder.frag_output.iter_mut().for_each(|(_, state)| {
            state.blend = BlendState::ALPHA_BLENDING.into();
          });
        }
      };

      builder.register::<ColorChannel>(base_color);
      builder.register::<EmissiveChannel>(emissive);
      builder.register::<MetallicChannel>(metallic);
      builder.register::<RoughnessChannel>(roughness * roughness);

      builder.register::<DefaultDisplay>((base_color, val(1.)));
      Ok(())
    })
  }
}
