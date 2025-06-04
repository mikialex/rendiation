use rendiation_lighting_transport::{EmissiveChannel, GlossinessChannel, SpecularChannel};
use rendiation_shader_library::normal_mapping::apply_normal_mapping_conditional;

use crate::*;

#[derive(Default)]
pub struct PbrSGMaterialDefaultRenderImplProvider {
  uniforms: QueryToken,
  tex_uniforms: QueryToken,
}

impl QueryBasedFeature<Box<dyn GLESModelMaterialRenderImpl>>
  for PbrSGMaterialDefaultRenderImplProvider
{
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    self.uniforms = qcx.register_multi_updater(pbr_sg_material_uniforms(cx));
    self.tex_uniforms = qcx.register_multi_updater(pbr_sg_material_tex_uniforms(cx));
  }
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.uniforms);
    qcx.deregister(&mut self.tex_uniforms);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn GLESModelMaterialRenderImpl> {
    Box::new(PbrSGMaterialDefaultRenderImpl {
      material_access: global_entity_component_of::<StandardModelRefPbrSGMaterial>()
        .read_foreign_key(),
      uniforms: cx.take_multi_updater_updated(self.uniforms).unwrap(),
      tex_uniforms: cx.take_multi_updater_updated(self.tex_uniforms).unwrap(),
      alpha_mode: global_entity_component_of().read(),
      albedo_tex_sampler: TextureSamplerIdView::read_from_global(),
      specular_glossiness_tex_sampler: TextureSamplerIdView::read_from_global(),
      emissive_tex_sampler: TextureSamplerIdView::read_from_global(),
      normal_tex_sampler: TextureSamplerIdView::read_from_global(),
    })
  }
}

struct PbrSGMaterialDefaultRenderImpl {
  material_access: ForeignKeyReadView<StandardModelRefPbrSGMaterial>,
  uniforms: LockReadGuardHolder<PbrSGMaterialUniforms>,
  tex_uniforms: LockReadGuardHolder<PbrSGMaterialTexUniforms>,
  alpha_mode: ComponentReadView<AlphaModeOf<PbrSGMaterialAlphaConfig>>,
  albedo_tex_sampler: TextureSamplerIdView<PbrSGMaterialAlbedoAlphaTex>,
  specular_glossiness_tex_sampler: TextureSamplerIdView<PbrSGMaterialSpecularGlossinessTex>,
  emissive_tex_sampler: TextureSamplerIdView<PbrSGMaterialEmissiveTex>,
  normal_tex_sampler: TextureSamplerIdView<NormalTexSamplerOf<PbrSGMaterialNormalInfo>>,
}

impl GLESModelMaterialRenderImpl for PbrSGMaterialDefaultRenderImpl {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let idx = self.material_access.get(idx)?;
    let r = PhysicalSpecularGlossinessMaterialGPU {
      uniform: self.uniforms.get(&idx)?,
      alpha_mode: self.alpha_mode.get_value(idx)?,
      albedo_alpha_tex_sampler: self.albedo_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
      specular_glossiness_tex_sampler: self
        .specular_glossiness_tex_sampler
        .get_pair(idx)
        .unwrap_or(EMPTY_H),
      emissive_tex_sampler: self.emissive_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
      normal_tex_sampler: self.normal_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
      texture_uniforms: self.tex_uniforms.get(&idx)?,
      binding_sys: cx,
    };
    let r = Box::new(r) as Box<dyn RenderComponent + '_>;
    Some(r)
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct PhysicalSpecularGlossinessMaterialUniform {
  pub albedo: Vec3<f32>,
  pub specular: Vec3<f32>,
  pub emissive: Vec3<f32>,
  pub glossiness: f32,
  pub normal_mapping_scale: f32,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}
type Uniform = PhysicalSpecularGlossinessMaterialUniform;

pub type PbrSGMaterialUniforms = UniformUpdateContainer<EntityHandle<PbrSGMaterialEntity>, Uniform>;
pub fn pbr_sg_material_uniforms(cx: &GPU) -> PbrSGMaterialUniforms {
  let albedo = global_watch()
    .watch::<PbrSGMaterialAlbedoComponent>()
    .into_query_update_uniform(offset_of!(Uniform, albedo), cx);

  let emissive = global_watch()
    .watch::<PbrSGMaterialEmissiveComponent>()
    .into_query_update_uniform(offset_of!(Uniform, emissive), cx);

  let normal_mapping_scale = global_watch()
    .watch::<NormalScaleOf<PbrSGMaterialNormalInfo>>()
    .into_query_update_uniform(offset_of!(Uniform, normal_mapping_scale), cx);

  let glossiness = global_watch()
    .watch::<PbrSGMaterialGlossinessComponent>()
    .into_query_update_uniform(offset_of!(Uniform, glossiness), cx);

  let alpha = global_watch()
    .watch::<AlphaOf<PbrSGMaterialAlphaConfig>>()
    .into_query_update_uniform(offset_of!(Uniform, alpha), cx);

  PbrSGMaterialUniforms::default()
    .with_source(albedo)
    .with_source(emissive)
    .with_source(normal_mapping_scale)
    .with_source(glossiness)
    .with_source(alpha)
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct PhysicalSpecularGlossinessMaterialTextureHandlesUniform {
  pub albedo_alpha_texture: TextureSamplerHandlePair,
  pub specular_texture: TextureSamplerHandlePair,
  pub emissive_texture: TextureSamplerHandlePair,
  pub glossiness_texture: TextureSamplerHandlePair,
  pub normal_texture: TextureSamplerHandlePair,
}
type TexUniform = PhysicalSpecularGlossinessMaterialTextureHandlesUniform;

pub type PbrSGMaterialTexUniforms =
  UniformUpdateContainer<EntityHandle<PbrSGMaterialEntity>, TexUniform>;
pub fn pbr_sg_material_tex_uniforms(cx: &GPU) -> PbrSGMaterialTexUniforms {
  let c = PbrSGMaterialTexUniforms::default();

  let albedo_alpha = offset_of!(TexUniform, albedo_alpha_texture);
  let emissive = offset_of!(TexUniform, emissive_texture);
  let specular = offset_of!(TexUniform, specular_texture);
  let normal = offset_of!(TexUniform, normal_texture);
  let c = add_tex_watcher::<PbrSGMaterialAlbedoAlphaTex, _>(c, albedo_alpha, cx);
  let c = add_tex_watcher::<PbrSGMaterialEmissiveTex, _>(c, emissive, cx);
  let c = add_tex_watcher::<PbrSGMaterialSpecularGlossinessTex, _>(c, specular, cx);
  add_tex_watcher::<NormalTexSamplerOf<PbrSGMaterialNormalInfo>, _>(c, normal, cx)
}

pub fn pbr_sg_material_pipeline_hash(
) -> impl ReactiveQuery<Key = EntityHandle<PbrSGMaterialEntity>, Value = AlphaMode> {
  global_watch().watch::<AlphaModeOf<PbrSGMaterialAlphaConfig>>()
}

pub struct PhysicalSpecularGlossinessMaterialGPU<'a> {
  pub uniform: &'a UniformBufferDataView<PhysicalSpecularGlossinessMaterialUniform>,
  pub alpha_mode: AlphaMode,
  // these idx is only useful in per object binding mode
  pub albedo_alpha_tex_sampler: (u32, u32),
  pub specular_glossiness_tex_sampler: (u32, u32),
  pub emissive_tex_sampler: (u32, u32),
  pub normal_tex_sampler: (u32, u32),
  // no matter if we using indirect texture binding, this uniform is required for checking the
  // texture if is exist in shader
  pub texture_uniforms:
    &'a UniformBufferDataView<PhysicalSpecularGlossinessMaterialTextureHandlesUniform>,
  pub binding_sys: &'a GPUTextureBindingSystem,
}

impl ShaderHashProvider for PhysicalSpecularGlossinessMaterialGPU<'_> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
  shader_hash_type_id! {PhysicalSpecularGlossinessMaterialGPU<'static>}
}

impl ShaderPassBuilder for PhysicalSpecularGlossinessMaterialGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
    ctx.binding.bind(self.texture_uniforms);
    setup_tex(ctx, self.binding_sys, self.albedo_alpha_tex_sampler);
    setup_tex(ctx, self.binding_sys, self.specular_glossiness_tex_sampler);
    setup_tex(ctx, self.binding_sys, self.emissive_tex_sampler);
    setup_tex(ctx, self.binding_sys, self.normal_tex_sampler);
  }
}

impl GraphicsShaderProvider for PhysicalSpecularGlossinessMaterialGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let uniform = binding.bind_by(&self.uniform).load().expand();
      let tex_uniform = binding.bind_by(&self.texture_uniforms).load().expand();
      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();

      let mut alpha = uniform.alpha;

      let mut base_color = uniform.albedo;

      let albedo_alpha = bind_and_sample(
        self.binding_sys,
        binding,
        builder.registry(),
        self.albedo_alpha_tex_sampler,
        tex_uniform.albedo_alpha_texture,
        uv,
        val(Vec4::one()),
      );
      alpha *= albedo_alpha.w();
      base_color *= albedo_alpha.xyz();

      let mut specular = uniform.specular;
      let specular_glossiness = bind_and_sample(
        self.binding_sys,
        binding,
        builder.registry(),
        self.specular_glossiness_tex_sampler,
        tex_uniform.specular_texture,
        uv,
        val(Vec4::one()),
      );
      specular *= specular_glossiness.xyz();
      let glossiness = uniform.glossiness * specular_glossiness.w();

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
      builder.register::<SpecularChannel>(specular);
      builder.register::<EmissiveChannel>(emissive);
      builder.register::<GlossinessChannel>(glossiness * glossiness);

      builder.register::<DefaultDisplay>((albedo_alpha.xyz(), val(1.)));
      builder.insert_type_tag::<PbrSGMaterialTag>();
    })
  }
}
