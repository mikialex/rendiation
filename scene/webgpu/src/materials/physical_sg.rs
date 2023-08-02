use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct PhysicalSpecularGlossinessMaterialUniform {
  pub albedo: Vec3<f32>,
  pub albedo_texture: TextureSamplerHandlePair,
  pub specular: Vec3<f32>,
  pub specular_texture: TextureSamplerHandlePair,
  pub emissive: Vec3<f32>,
  pub emissive_texture: TextureSamplerHandlePair,
  pub glossiness: f32,
  pub glossiness_texture: TextureSamplerHandlePair,
  pub normal_mapping_scale: f32,
  pub normal_texture: TextureSamplerHandlePair,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}

impl ShaderHashProvider for PhysicalSpecularGlossinessMaterialGPU {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
}

#[pin_project::pin_project]
pub struct PhysicalSpecularGlossinessMaterialGPU {
  uniform: UniformBufferDataView<PhysicalSpecularGlossinessMaterialUniform>,
  albedo_texture: ReactiveGPUTextureSamplerPair,
  specular_texture: ReactiveGPUTextureSamplerPair,
  glossiness_texture: ReactiveGPUTextureSamplerPair,
  emissive_texture: ReactiveGPUTextureSamplerPair,
  normal_texture: ReactiveGPUTextureSamplerPair,
  alpha_mode: AlphaMode,
}

impl Stream for PhysicalSpecularGlossinessMaterialGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let mut r = RenderComponentDeltaFlag::default();
    poll_update_texture_handle_uniform!(this, albedo_texture, cx, r);
    poll_update_texture_handle_uniform!(this, specular_texture, cx, r);
    poll_update_texture_handle_uniform!(this, glossiness_texture, cx, r);
    poll_update_texture_handle_uniform!(this, emissive_texture, cx, r);
    poll_update_texture_handle_uniform!(this, normal_texture, cx, r);
    r.into_poll()
  }
}

impl ShaderPassBuilder for PhysicalSpecularGlossinessMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform);
    self.albedo_texture.setup_pass(ctx);
    self.specular_texture.setup_pass(ctx);
    self.glossiness_texture.setup_pass(ctx);
    self.emissive_texture.setup_pass(ctx);
    self.normal_texture.setup_pass(ctx);
  }
}

impl ShaderGraphProvider for PhysicalSpecularGlossinessMaterialGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.context.insert(
      ShadingSelection.type_id(),
      Box::new(&PhysicalShading as &dyn LightableSurfaceShadingDyn),
    );

    builder.fragment(|builder, binding| {
      let uniform = binding.uniform_by(&self.uniform).expand();
      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();

      let mut alpha = uniform.alpha;

      let mut albedo = uniform.albedo;
      let albedo_tex = self.albedo_texture.uniform_and_sample(
        binding,
        builder.registry(),
        uniform.albedo_texture,
        uv,
      );
      alpha *= albedo_tex.w();
      albedo *= albedo_tex.xyz();

      let mut specular = uniform.specular;
      specular *= self
        .specular_texture
        .uniform_and_sample(binding, builder.registry(), uniform.specular_texture, uv)
        .xyz();

      let mut glossiness = uniform.glossiness;
      glossiness *= self
        .specular_texture
        .uniform_and_sample(binding, builder.registry(), uniform.glossiness_texture, uv)
        .x();

      let mut emissive = uniform.emissive;
      emissive *= self
        .emissive_texture
        .uniform_and_sample(binding, builder.registry(), uniform.emissive_texture, uv)
        .xyz();

      let (normal_sample, enabled) = self.normal_texture.uniform_and_sample_enabled(
        binding,
        builder.registry(),
        uniform.normal_texture,
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
          let alpha = alpha
            .less_than(uniform.alpha_cutoff)
            .select(consts(0.), alpha);
          builder.register::<AlphaChannel>(alpha);
          builder.register::<AlphaCutChannel>(uniform.alpha_cutoff);
        }
        AlphaMode::Blend => {
          builder.register::<AlphaChannel>(alpha);
          builder.frag_output.iter_mut().for_each(|(_, state)| {
            state.blend = webgpu::BlendState::ALPHA_BLENDING.into();
          });
        }
      };

      builder.register::<ColorChannel>(albedo);
      builder.register::<SpecularChannel>(specular);
      builder.register::<EmissiveChannel>(emissive);
      builder.register::<GlossinessChannel>(glossiness);

      builder.register::<DefaultDisplay>((albedo, 1.));
      Ok(())
    })
  }
}

use PhysicalSpecularGlossinessMaterialDelta as PD;

impl ReactiveRenderComponentSource for PhysicalSpecularGlossinessMaterialReactiveGPU {
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent {
    self.as_ref() as &dyn ReactiveRenderComponent
  }
}

type PhysicalSpecularGlossinessMaterialReactiveGPU = impl AsRef<RenderComponentCell<PhysicalSpecularGlossinessMaterialGPU>>
  + Stream<Item = RenderComponentDeltaFlag>;

impl WebGPUMaterial for PhysicalSpecularGlossinessMaterial {
  type ReactiveGPU = PhysicalSpecularGlossinessMaterialReactiveGPU;

  fn create_reactive_gpu(
    source: &SceneItemRef<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU {
    let m = source.read();

    let uniform = build_shader_uniform(&m);
    let uniform = create_uniform(uniform, &ctx.gpu.device);

    let albedo_texture = ctx.build_reactive_texture_sampler_pair(m.albedo_texture.as_ref());
    let glossiness_texture = ctx.build_reactive_texture_sampler_pair(m.glossiness_texture.as_ref());
    let specular_texture = ctx.build_reactive_texture_sampler_pair(m.specular_texture.as_ref());
    let emissive_texture = ctx.build_reactive_texture_sampler_pair(m.emissive_texture.as_ref());

    let normal_texture =
      ctx.build_reactive_texture_sampler_pair(m.normal_texture.as_ref().map(|t| &t.content));

    let gpu = PhysicalSpecularGlossinessMaterialGPU {
      uniform,
      albedo_texture,
      specular_texture,
      glossiness_texture,
      emissive_texture,
      normal_texture,
      alpha_mode: m.alpha_mode,
    };

    let state = RenderComponentCell::new(gpu);

    let weak_material = source.downgrade();
    let ctx = ctx.clone();

    let uniform_any_change = source
      .single_listen_by::<()>(all_delta_with(false, then_some(is_uniform_changed)))
      .map(|_| UniformChangePicked::UniformChange);

    let all = source
      .unbound_listen_by(all_delta_no_init)
      .map(UniformChangePicked::Origin);

    futures::stream::select(uniform_any_change, all).fold_signal_flatten(
      state,
      move |delta, state| match delta {
        UniformChangePicked::UniformChange => {
          if let Some(m) = weak_material.upgrade() {
            state.uniform.set(build_shader_uniform(&m.read()));
            state.uniform.upload(&ctx.gpu.queue)
          }
          RenderComponentDeltaFlag::ContentRef.into()
        }
        UniformChangePicked::Origin(delta) => match delta {
          PD::alpha_mode(_) => RenderComponentDeltaFlag::ShaderHash,
          PD::albedo_texture(t) => apply_tex_pair_delta(t, &mut state.albedo_texture, &ctx),
          PD::glossiness_texture(t) => apply_tex_pair_delta(t, &mut state.glossiness_texture, &ctx),
          PD::specular_texture(t) => apply_tex_pair_delta(t, &mut state.specular_texture, &ctx),
          PD::emissive_texture(t) => apply_tex_pair_delta(t, &mut state.emissive_texture, &ctx),
          PD::normal_texture(t) => apply_normal_map_delta(t, &mut state.normal_texture, &ctx),
          _ => RenderComponentDeltaFlag::Content, // handled in uniform
        }
        .into(),
      },
    )
  }

  fn is_transparent(&self) -> bool {
    matches!(self.alpha_mode, AlphaMode::Blend)
  }
}

fn build_shader_uniform(
  m: &PhysicalSpecularGlossinessMaterial,
) -> PhysicalSpecularGlossinessMaterialUniform {
  let mut r = PhysicalSpecularGlossinessMaterialUniform {
    albedo: m.albedo,
    specular: m.specular,
    emissive: m.emissive,
    glossiness: m.glossiness,
    normal_mapping_scale: 1.,
    alpha_cutoff: m.alpha_cutoff,
    alpha: m.alpha,
    ..Zeroable::zeroed()
  };

  if let Some(normal_texture) = &m.normal_texture {
    r.normal_mapping_scale = normal_texture.scale;
  };

  r
}

fn is_uniform_changed(d: DeltaOf<PhysicalSpecularGlossinessMaterial>) -> bool {
  matches!(
    d,
    PD::albedo(_)
      | PD::specular(_)
      | PD::glossiness(_)
      | PD::emissive(_)
      | PD::alpha(_)
      | PD::alpha_cutoff(_)
      | PD::normal_texture(_) // normal map scale
  )
}
