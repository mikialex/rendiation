use rendiation_scene_rendering_gpu_gles::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

use crate::*;

pub fn use_occ_material_uniforms(cx: &mut QueryGPUHookCx) -> Option<OccStyleMaterialGlesRenderer> {
  let uniforms = cx.use_uniform_buffers();

  cx.use_changes::<OccStyleMaterialDiffuse>()
    .map(|changes| changes.collective_map(srgb4_to_linear4))
    .update_uniforms(
      &uniforms,
      offset_of!(OccStyleMaterialUniform, diffuse),
      cx.gpu,
    );

  cx.use_changes::<OccStyleMaterialSpecular>()
    .update_uniforms(
      &uniforms,
      offset_of!(OccStyleMaterialUniform, specular),
      cx.gpu,
    );

  cx.use_changes::<OccStyleMaterialShiness>().update_uniforms(
    &uniforms,
    offset_of!(OccStyleMaterialUniform, shiness),
    cx.gpu,
  );

  cx.use_changes::<OccStyleMaterialEmissive>()
    .update_uniforms(
      &uniforms,
      offset_of!(OccStyleMaterialUniform, emissive),
      cx.gpu,
    );

  let tex_uniforms = cx.use_uniform_buffers();

  let diffuse_tex = offset_of!(OccStyleMaterialTextureHandlesUniform, diffuse_texture);
  use_tex_watcher::<OccStyleMaterialDiffuseTex, _>(cx, diffuse_tex, &tex_uniforms);

  cx.when_render(|| OccStyleMaterialGlesRenderer {
    material_access: read_global_db_foreign_key(),
    transparent: read_global_db_component(),
    effect_access: read_global_db_foreign_key(),
    shade_type: read_global_db_component(),
    uniforms: uniforms.make_read_holder(),
    tex_uniforms: tex_uniforms.make_read_holder(),
    diffuse_tex_sampler: TextureSamplerIdView::read_from_global(),
  })
}

pub struct OccStyleMaterialGlesRenderer {
  material_access: ForeignKeyReadView<StdModelOccStyleMaterialPayload>,
  transparent: ComponentReadView<OccStyleMaterialTransparent>,
  effect_access: ForeignKeyReadView<OccStyleMaterialEffect>,
  shade_type: ComponentReadView<OccStyleEffectShadeType>,
  uniforms: LockReadGuardHolder<OccStyleMaterialUniforms>,
  tex_uniforms: LockReadGuardHolder<OccStyleMaterialTexUniforms>,
  diffuse_tex_sampler: TextureSamplerIdView<OccStyleMaterialDiffuseTex>,
}

impl GLESModelMaterialRenderImpl for OccStyleMaterialGlesRenderer {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let idx = self.material_access.get(idx)?;
    let transparent = self.transparent.get_value(idx)?;
    let effect = self.effect_access.get(idx)?;
    let shade_type = self.shade_type.get_value(effect)?;
    Some(Box::new(OccStyleMaterialGPU {
      uniform: self.uniforms.get(&idx.alloc_index())?,
      tex_uniform: self.tex_uniforms.get(&idx.alloc_index())?,
      diffuse_tex_sampler: self.diffuse_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
      binding_sys: cx,
      transparent,
      shade_type,
    }))
  }
}

type OccStyleMaterialUniforms = UniformBufferCollectionRaw<u32, OccStyleMaterialUniform>;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
struct OccStyleMaterialUniform {
  pub diffuse: Vec4<f32>,
  pub specular: Vec3<f32>,
  pub shiness: f32,
  pub emissive: Vec3<f32>,
}

type OccStyleMaterialTexUniforms =
  UniformBufferCollectionRaw<u32, OccStyleMaterialTextureHandlesUniform>;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
struct OccStyleMaterialTextureHandlesUniform {
  pub diffuse_texture: TextureSamplerHandlePair,
}

#[derive(Clone)]
pub struct OccStyleMaterialGPU<'a> {
  uniform: &'a UniformBufferDataView<OccStyleMaterialUniform>,
  tex_uniform: &'a UniformBufferDataView<OccStyleMaterialTextureHandlesUniform>,
  diffuse_tex_sampler: (u32, u32),
  binding_sys: &'a GPUTextureBindingSystem,
  pub transparent: bool,
  pub shade_type: OccStyleEffectType,
}

impl ShaderHashProvider for OccStyleMaterialGPU<'_> {
  shader_hash_type_id! {OccStyleMaterialGPU<'static>}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.transparent.hash(hasher);
    self.shade_type.hash(hasher);
  }
}

impl GraphicsShaderProvider for OccStyleMaterialGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      if self.transparent {
        builder.frag_output.iter_mut().for_each(|p| {
          if p.is_blendable() {
            p.states.blend = BlendState::ALPHA_BLENDING.into();
          }
        });
      }
    });
  }
}

impl ShaderPassBuilder for OccStyleMaterialGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
    ctx.binding.bind(self.tex_uniform);
    setup_tex(ctx, self.binding_sys, self.diffuse_tex_sampler);
  }
}
