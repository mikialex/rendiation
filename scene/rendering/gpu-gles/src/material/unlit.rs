use crate::*;

pub fn use_unlit_material_uniforms(cx: &mut QueryGPUHookCx) -> Option<UnlitMaterialGlesRender> {
  let uniform = cx.use_uniform_buffers::<EntityHandle<UnlitMaterialEntity>, UnlitMaterialUniform>(
    |source, cx| {
      let color = global_watch()
        .watch::<UnlitMaterialColorComponent>()
        .collective_map(srgb4_to_linear4)
        .into_query_update_uniform(offset_of!(UnlitMaterialUniform, color), cx);

      let alpha = global_watch()
        .watch::<AlphaOf<UnlitMaterialAlphaConfig>>()
        .into_query_update_uniform(offset_of!(UnlitMaterialUniform, alpha), cx);

      let alpha_cutoff = global_watch()
        .watch::<AlphaCutoffOf<UnlitMaterialAlphaConfig>>()
        .into_query_update_uniform(offset_of!(UnlitMaterialUniform, alpha_cutoff), cx);

      source
        .with_source(color)
        .with_source(alpha)
        .with_source(alpha_cutoff)
    },
  );

  let tex_uniform = cx
    .use_uniform_buffers::<EntityHandle<UnlitMaterialEntity>, UnlitMaterialTextureHandlesUniform>(
      |source, cx| {
        let color_alpha_texture =
          offset_of!(UnlitMaterialTextureHandlesUniform, color_alpha_texture);
        add_tex_watcher::<UnlitMaterialColorAlphaTex, _>(source, color_alpha_texture, cx)
      },
    );

  cx.when_render(|| UnlitMaterialGlesRender {
    material_access: global_entity_component_of::<StandardModelRefUnlitMaterial>()
      .read_foreign_key(),
    uniforms: uniform.unwrap(),
    texture_uniforms: tex_uniform.unwrap(),
    color_tex_sampler: TextureSamplerIdView::read_from_global(),
    alpha_mode: global_entity_component_of().read(),
  })
}

pub struct UnlitMaterialGlesRender {
  material_access: ForeignKeyReadView<StandardModelRefUnlitMaterial>,
  uniforms: LockReadGuardHolder<UnlitMaterialUniforms>,
  texture_uniforms: LockReadGuardHolder<UnlitMaterialTexUniforms>,
  color_tex_sampler: TextureSamplerIdView<UnlitMaterialColorAlphaTex>,
  alpha_mode: ComponentReadView<AlphaModeOf<UnlitMaterialAlphaConfig>>,
}

impl GLESModelMaterialRenderImpl for UnlitMaterialGlesRender {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let idx = self.material_access.get(idx)?;
    Some(Box::new(UnlitMaterialGPU {
      uniform: self.uniforms.get(&idx)?,
      alpha_mode: self.alpha_mode.get_value(idx)?,
      binding_sys: cx,
      tex_uniform: self.texture_uniforms.get(&idx)?,
      color_alpha_tex_sampler: self.color_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
    }))
  }
}

type UnlitMaterialUniforms =
  UniformUpdateContainer<EntityHandle<UnlitMaterialEntity>, UnlitMaterialUniform>;

type TexUniform = UnlitMaterialTextureHandlesUniform;
type UnlitMaterialTexUniforms =
  UniformUpdateContainer<EntityHandle<UnlitMaterialEntity>, TexUniform>;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
struct UnlitMaterialUniform {
  pub color: Vec4<f32>,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}
#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
struct UnlitMaterialTextureHandlesUniform {
  pub color_alpha_texture: TextureSamplerHandlePair,
}

pub fn unlit_material_pipeline_hash(
) -> impl ReactiveQuery<Key = EntityHandle<UnlitMaterialEntity>, Value = AlphaMode> {
  global_watch().watch::<AlphaModeOf<UnlitMaterialAlphaConfig>>()
}

#[derive(Clone)]
pub struct UnlitMaterialGPU<'a> {
  uniform: &'a UniformBufferDataView<UnlitMaterialUniform>,
  tex_uniform: &'a UniformBufferDataView<UnlitMaterialTextureHandlesUniform>,
  alpha_mode: AlphaMode,
  color_alpha_tex_sampler: (u32, u32),
  binding_sys: &'a GPUTextureBindingSystem,
}

impl ShaderHashProvider for UnlitMaterialGPU<'_> {
  shader_hash_type_id! {UnlitMaterialGPU<'static>}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
}

impl GraphicsShaderProvider for UnlitMaterialGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    // allow this material to be used with none uv geometry provider
    builder.vertex(|builder, _| {
      if builder.try_query::<GeometryUV>().is_none() {
        builder.register::<GeometryUV>(val(Vec2::zero()));
      }
    });
    builder.fragment(|builder, binding| {
      let uniform = binding.bind_by(&self.uniform).load().expand();
      let tex_uniform = binding.bind_by(&self.tex_uniform).load().expand();
      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();

      let color_alpha_tex = bind_and_sample(
        self.binding_sys,
        binding,
        builder.registry(),
        self.color_alpha_tex_sampler,
        tex_uniform.color_alpha_texture,
        uv,
        val(Vec4::one()),
      );

      builder.register::<DefaultDisplay>(uniform.color * color_alpha_tex);

      ShaderAlphaConfig {
        alpha_mode: self.alpha_mode,
        alpha_cutoff: uniform.alpha_cutoff,
        alpha: uniform.alpha * color_alpha_tex.w(),
      }
      .apply(builder);
    })
  }
}

impl ShaderPassBuilder for UnlitMaterialGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
    ctx.binding.bind(self.tex_uniform);
    setup_tex(ctx, self.binding_sys, self.color_alpha_tex_sampler);
  }
}
