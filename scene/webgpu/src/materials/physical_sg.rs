use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct PhysicalSpecularGlossinessMaterialUniform {
  pub albedo: Vec3<f32>,
  pub specular: Vec3<f32>,
  pub emissive: Vec3<f32>,
  pub glossiness: f32,
  pub normal_mapping_scale: f32,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}

impl ShaderHashProvider for PhysicalSpecularGlossinessMaterialGPU {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    // todo optimize for reduce shader permutation
    self.albedo_texture.is_some().hash(hasher);
    self.specular_texture.is_some().hash(hasher);
    self.glossiness_texture.is_some().hash(hasher);
    self.emissive_texture.is_some().hash(hasher);
    self.alpha_mode.hash(hasher);
  }
}

#[pin_project::pin_project]
pub struct PhysicalSpecularGlossinessMaterialGPU {
  uniform: UniformBufferDataView<PhysicalSpecularGlossinessMaterialUniform>,
  albedo_texture: Option<ReactiveGPUTextureSamplerPair>,
  specular_texture: Option<ReactiveGPUTextureSamplerPair>,
  glossiness_texture: Option<ReactiveGPUTextureSamplerPair>,
  emissive_texture: Option<ReactiveGPUTextureSamplerPair>,
  normal_texture: Option<ReactiveGPUTextureSamplerPair>,
  alpha_mode: AlphaMode,
}

impl Stream for PhysicalSpecularGlossinessMaterialGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    early_return_option_ready!(this.albedo_texture, cx);
    early_return_option_ready!(this.specular_texture, cx);
    early_return_option_ready!(this.glossiness_texture, cx);
    early_return_option_ready!(this.emissive_texture, cx);
    early_return_option_ready!(this.normal_texture, cx);
    Poll::Pending
  }
}

impl ShaderPassBuilder for PhysicalSpecularGlossinessMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform);
    if let Some(t) = self.albedo_texture.as_ref() {
      t.setup_pass(ctx)
    }
    if let Some(t) = self.specular_texture.as_ref() {
      t.setup_pass(ctx)
    }
    if let Some(t) = self.glossiness_texture.as_ref() {
      t.setup_pass(ctx)
    }
    if let Some(t) = self.emissive_texture.as_ref() {
      t.setup_pass(ctx)
    }
    if let Some(t) = self.normal_texture.as_ref() {
      t.setup_pass(ctx)
    }
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

      let albedo = if let Some(tex) = &self.albedo_texture {
        let sample = tex.uniform_and_sample(binding, uv);
        alpha *= sample.w();
        sample.xyz() * uniform.albedo
      } else {
        uniform.albedo
      };

      let specular = if let Some(tex) = &self.specular_texture {
        tex.uniform_and_sample(binding, uv).xyz() * uniform.specular
      } else {
        uniform.specular
      };

      let glossiness = if let Some(tex) = &self.glossiness_texture {
        tex.uniform_and_sample(binding, uv).x() * uniform.glossiness
      } else {
        uniform.glossiness
      };

      let emissive = if let Some(tex) = &self.emissive_texture {
        tex.uniform_and_sample(binding, uv).x() * uniform.emissive
      } else {
        uniform.emissive
      };

      if let Some(tex) = &self.normal_texture {
        let normal_sample = tex.uniform_and_sample(binding, uv).xyz();
        apply_normal_mapping(builder, normal_sample, uv, uniform.normal_mapping_scale);
      }

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
    let uniform = create_uniform2(uniform, &ctx.gpu.device);

    let albedo_texture = m
      .albedo_texture
      .as_ref()
      .map(|t| ctx.build_reactive_texture_sampler_pair(t));

    let glossiness_texture = m
      .glossiness_texture
      .as_ref()
      .map(|t| ctx.build_reactive_texture_sampler_pair(t));

    let specular_texture = m
      .specular_texture
      .as_ref()
      .map(|t| ctx.build_reactive_texture_sampler_pair(t));

    let emissive_texture = m
      .emissive_texture
      .as_ref()
      .map(|t| ctx.build_reactive_texture_sampler_pair(t));

    let normal_texture = m
      .normal_texture
      .as_ref()
      .map(|t| ctx.build_reactive_texture_sampler_pair(&t.content));

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
