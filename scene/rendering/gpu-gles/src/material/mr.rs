use rendiation_lighting_transport::{EmissiveChannel, MetallicChannel, RoughnessChannel};
use rendiation_shader_library::normal_mapping::apply_normal_mapping_conditional;

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

pub type PbrMRMaterialUniforms = UniformUpdateContainer<EntityHandle<PbrMRMaterialEntity>, Uniform>;
pub fn pbr_mr_material_uniforms(cx: &GPU) -> PbrMRMaterialUniforms {
  let base_color = global_watch()
    .watch::<PbrMRMaterialBaseColorComponent>()
    .into_query_update_uniform(offset_of!(Uniform, base_color), cx);

  let emissive = global_watch()
    .watch::<PbrMRMaterialEmissiveComponent>()
    .into_query_update_uniform(offset_of!(Uniform, emissive), cx);

  let normal_mapping_scale = global_watch()
    .watch::<NormalScaleOf<PbrMRMaterialNormalInfo>>()
    .into_query_update_uniform(offset_of!(Uniform, normal_mapping_scale), cx);

  let roughness = global_watch()
    .watch::<PbrMRMaterialRoughnessComponent>()
    .into_query_update_uniform(offset_of!(Uniform, roughness), cx);

  let metallic = global_watch()
    .watch::<PbrMRMaterialMetallicComponent>()
    .into_query_update_uniform(offset_of!(Uniform, metallic), cx);

  let alpha = global_watch()
    .watch::<AlphaOf<PbrMRMaterialAlphaConfig>>()
    .into_query_update_uniform(offset_of!(Uniform, alpha), cx);

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
  pub base_color_alpha_texture: TextureSamplerHandlePair,
  pub emissive_texture: TextureSamplerHandlePair,
  pub metallic_roughness_texture: TextureSamplerHandlePair,
  pub normal_texture: TextureSamplerHandlePair,
}
type TexUniform = PhysicalMetallicRoughnessMaterialTextureHandlesUniform;

pub type PbrMRMaterialTexUniforms =
  UniformUpdateContainer<EntityHandle<PbrMRMaterialEntity>, TexUniform>;
pub fn pbr_mr_material_tex_uniforms(cx: &GPU) -> PbrMRMaterialTexUniforms {
  let c = PbrMRMaterialTexUniforms::default();

  let base_color_alpha = offset_of!(TexUniform, base_color_alpha_texture);
  let emissive = offset_of!(TexUniform, emissive_texture);
  let metallic_roughness = offset_of!(TexUniform, metallic_roughness_texture);
  let normal = offset_of!(TexUniform, normal_texture);
  let c = add_tex_watcher::<PbrMRMaterialBaseColorAlphaTex, _>(c, base_color_alpha, cx);
  let c = add_tex_watcher::<PbrMRMaterialEmissiveTex, _>(c, emissive, cx);
  let c = add_tex_watcher::<PbrMRMaterialMetallicRoughnessTex, _>(c, metallic_roughness, cx);
  add_tex_watcher::<NormalTexSamplerOf<PbrMRMaterialNormalInfo>, _>(c, normal, cx)
}

pub fn pbr_mr_material_pipeline_hash(
) -> impl ReactiveQuery<Key = EntityHandle<PbrMRMaterialEntity>, Value = AlphaMode> {
  global_watch().watch::<AlphaModeOf<PbrMRMaterialAlphaConfig>>()
}

pub struct PhysicalMetallicRoughnessMaterialGPU<'a> {
  pub uniform: &'a UniformBufferDataView<PhysicalMetallicRoughnessMaterialUniform>,
  pub alpha_mode: AlphaMode,
  // these idx is only useful in per object binding mode
  pub base_color_alpha_tex_sampler: (u32, u32),
  pub mr_tex_sampler: (u32, u32),
  pub emissive_tex_sampler: (u32, u32),
  pub normal_tex_sampler: (u32, u32),
  // no matter if we using indirect texture binding, this uniform is required for checking the
  // texture if is exist in shader
  pub texture_uniforms:
    &'a UniformBufferDataView<PhysicalMetallicRoughnessMaterialTextureHandlesUniform>,
  pub binding_sys: &'a GPUTextureBindingSystem,
}

impl ShaderHashProvider for PhysicalMetallicRoughnessMaterialGPU<'_> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
  shader_hash_type_id! {PhysicalMetallicRoughnessMaterialGPU<'static>}
}

impl ShaderPassBuilder for PhysicalMetallicRoughnessMaterialGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
    ctx.binding.bind(self.texture_uniforms);
    setup_tex(ctx, self.binding_sys, self.base_color_alpha_tex_sampler);
    setup_tex(ctx, self.binding_sys, self.mr_tex_sampler);
    setup_tex(ctx, self.binding_sys, self.emissive_tex_sampler);
    setup_tex(ctx, self.binding_sys, self.normal_tex_sampler);
  }
}

impl GraphicsShaderProvider for PhysicalMetallicRoughnessMaterialGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let uniform = binding.bind_by(&self.uniform).load().expand();
      let tex_uniform = binding.bind_by(&self.texture_uniforms).load().expand();

      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();

      let mut alpha = uniform.alpha;
      let mut base_color = uniform.base_color;

      let base_color_alpha_tex = bind_and_sample(
        self.binding_sys,
        binding,
        builder.registry(),
        self.base_color_alpha_tex_sampler,
        tex_uniform.base_color_alpha_texture,
        uv,
        val(Vec4::one()),
      );
      alpha *= base_color_alpha_tex.w();
      base_color *= base_color_alpha_tex.xyz();

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

      ShaderAlphaConfig {
        alpha_mode: self.alpha_mode,
        alpha_cutoff: uniform.alpha_cutoff,
        alpha,
      }
      .apply(builder);

      builder.register::<ColorChannel>(base_color);
      builder.register::<EmissiveChannel>(emissive);
      builder.register::<MetallicChannel>(metallic);
      builder.register::<RoughnessChannel>(roughness * roughness);

      builder.register::<DefaultDisplay>((base_color, val(1.)));
      builder.insert_type_tag::<PbrMRMaterialTag>();
    })
  }
}
