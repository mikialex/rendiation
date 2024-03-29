use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct PhysicalMetallicRoughnessMaterialUniform {
  pub base_color: Vec3<f32>,
  pub base_color_texture: TextureSamplerHandlePair,
  pub emissive: Vec3<f32>,
  pub emissive_texture: TextureSamplerHandlePair,
  pub roughness: f32,
  pub metallic: f32,
  pub metallic_roughness_texture: TextureSamplerHandlePair,
  pub reflectance: f32,
  pub normal_mapping_scale: f32,
  pub normal_texture: TextureSamplerHandlePair,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}

impl ShaderHashProvider for PhysicalMetallicRoughnessMaterialGPUImpl {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
}

#[pin_project::pin_project]
pub struct PhysicalMetallicRoughnessMaterialGPUImpl {
  uniform: UniformBufferCachedDataView<PhysicalMetallicRoughnessMaterialUniform>,
  base_color_texture: ReactiveGPUTextureSamplerPair,
  metallic_roughness_texture: ReactiveGPUTextureSamplerPair,
  emissive_texture: ReactiveGPUTextureSamplerPair,
  normal_texture: ReactiveGPUTextureSamplerPair,
  alpha_mode: AlphaMode,
  gpu: ResourceGPUCtx,
}

impl Stream for PhysicalMetallicRoughnessMaterialGPUImpl {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let mut r = RenderComponentDeltaFlag::default();
    let mut c = false;
    poll_update_texture_handle_uniform!(this, base_color_texture, cx, r, c);
    poll_update_texture_handle_uniform!(this, metallic_roughness_texture, cx, r, c);
    poll_update_texture_handle_uniform!(this, emissive_texture, cx, r, c);
    poll_update_texture_handle_uniform!(this, normal_texture, cx, r, c);
    if c {
      this.uniform.upload(&this.gpu.queue)
    }
    r.into_poll()
  }
}

impl ShaderPassBuilder for PhysicalMetallicRoughnessMaterialGPUImpl {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform);
    self.base_color_texture.setup_pass(ctx);
    self.metallic_roughness_texture.setup_pass(ctx);
    self.emissive_texture.setup_pass(ctx);
    self.normal_texture.setup_pass(ctx);
  }
}

impl GraphicsShaderProvider for PhysicalMetallicRoughnessMaterialGPUImpl {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.context.insert(
      ShadingSelection.type_id(),
      Box::new(&PhysicalShading as &dyn LightableSurfaceShadingDyn),
    );

    builder.fragment(|builder, binding| {
      let uniform = binding.bind_by(&self.uniform).load().expand();
      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();

      let mut alpha = uniform.alpha;
      let mut base_color = uniform.base_color;

      let base_color_tex = self.base_color_texture.bind_and_sample(
        binding,
        builder.registry(),
        uniform.base_color_texture,
        uv,
      );
      alpha *= base_color_tex.w();
      base_color *= base_color_tex.xyz();

      let mut metallic = uniform.metallic;
      let mut roughness = uniform.roughness;

      let metallic_roughness_tex = self.metallic_roughness_texture.bind_and_sample(
        binding,
        builder.registry(),
        uniform.metallic_roughness_texture,
        uv,
      );

      metallic *= metallic_roughness_tex.x();
      roughness *= metallic_roughness_tex.y();

      let mut emissive = uniform.emissive;
      emissive *= self
        .emissive_texture
        .bind_and_sample(binding, builder.registry(), uniform.emissive_texture, uv)
        .xyz();

      let (normal_sample, enabled) = self.normal_texture.bind_and_sample_enabled(
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

impl ReactiveRenderComponentSource for PhysicalMetallicRoughnessMaterialReactiveGPU {
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent {
    self.inner.as_ref() as &dyn ReactiveRenderComponent
  }
}

#[pin_project::pin_project]
pub struct PhysicalMetallicRoughnessMaterialReactiveGPU {
  #[pin]
  pub inner: PhysicalMetallicRoughnessMaterialInner,
}

impl Stream for PhysicalMetallicRoughnessMaterialReactiveGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.inner.poll_next(cx)
  }
}

pub type PhysicalMetallicRoughnessMaterialInner = impl AsRef<RenderComponentCell<PhysicalMetallicRoughnessMaterialGPUImpl>>
  + Stream<Item = RenderComponentDeltaFlag>;

use rendiation_shader_library::normal_mapping::apply_normal_mapping_conditional;
use PhysicalMetallicRoughnessMaterialDelta as PD;

impl WebGPUMaterial for PhysicalMetallicRoughnessMaterial {
  type ReactiveGPU = PhysicalMetallicRoughnessMaterialReactiveGPU;
  fn create_reactive_gpu(
    source: &IncrementalSignalPtr<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU {
    let m = source.read();

    let uniform = build_shader_uniform(&m);
    let uniform = create_uniform_with_cache(uniform, &ctx.gpu.device);

    let base_color_texture = ctx.build_reactive_texture_sampler_pair(m.base_color_texture.as_ref());
    let metallic_roughness_texture =
      ctx.build_reactive_texture_sampler_pair(m.metallic_roughness_texture.as_ref());
    let emissive_texture = ctx.build_reactive_texture_sampler_pair(m.emissive_texture.as_ref());

    let normal_texture =
      ctx.build_reactive_texture_sampler_pair(m.normal_texture.as_ref().map(|t| &t.content));

    let gpu = PhysicalMetallicRoughnessMaterialGPUImpl {
      uniform,
      base_color_texture,
      metallic_roughness_texture,
      emissive_texture,
      normal_texture,
      alpha_mode: m.alpha_mode,
      gpu: ctx.gpu.clone(),
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

    let inner = futures::stream::select(uniform_any_change, all).fold_signal_flatten(
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
          PD::alpha_mode(mode) => {
            state.alpha_mode = mode;
            RenderComponentDeltaFlag::ShaderHash
          }
          PD::base_color_texture(t) => apply_tex_pair_delta(t, &mut state.base_color_texture, &ctx),
          PD::metallic_roughness_texture(t) => {
            apply_tex_pair_delta(t, &mut state.metallic_roughness_texture, &ctx)
          }
          PD::emissive_texture(t) => apply_tex_pair_delta(t, &mut state.emissive_texture, &ctx),
          PD::normal_texture(t) => apply_normal_map_delta(t, &mut state.normal_texture, &ctx),
          _ => RenderComponentDeltaFlag::Content, // handled in uniform
        }
        .into(),
      },
    );

    PhysicalMetallicRoughnessMaterialReactiveGPU { inner }
  }

  fn is_transparent(&self) -> bool {
    matches!(self.alpha_mode, AlphaMode::Blend)
  }
}

fn build_shader_uniform(
  m: &PhysicalMetallicRoughnessMaterial,
) -> PhysicalMetallicRoughnessMaterialUniform {
  let mut r = PhysicalMetallicRoughnessMaterialUniform {
    base_color: m.base_color,
    roughness: m.roughness,
    emissive: m.emissive,
    metallic: m.metallic,
    reflectance: m.reflectance,
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

fn is_uniform_changed(d: DeltaOf<PhysicalMetallicRoughnessMaterial>) -> bool {
  matches!(
    d,
    PD::base_color(_)
      | PD::roughness(_)
      | PD::metallic(_)
      | PD::reflectance(_)
      | PD::emissive(_)
      | PD::alpha(_)
      | PD::alpha_cutoff(_)
      | PD::normal_texture(_) // normal map scale
  )
}
