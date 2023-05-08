use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
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

impl ShaderHashProvider for PhysicalMetallicRoughnessMaterialGPU {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    // todo optimize for reduce shader permutation
    self.base_color_texture.is_some().hash(hasher);
    self.metallic_roughness_texture.is_some().hash(hasher);
    self.emissive_texture.is_some().hash(hasher);
    self.alpha_mode.hash(hasher);
  }
}

#[pin_project::pin_project]
pub struct PhysicalMetallicRoughnessMaterialGPU {
  uniform: UniformBufferDataView<PhysicalMetallicRoughnessMaterialUniform>,
  base_color_texture: Option<ReactiveGPUTextureSamplerPair>,
  metallic_roughness_texture: Option<ReactiveGPUTextureSamplerPair>,
  emissive_texture: Option<ReactiveGPUTextureSamplerPair>,
  normal_texture: Option<ReactiveGPUTextureSamplerPair>,
  alpha_mode: AlphaMode,
}

impl Stream for PhysicalMetallicRoughnessMaterialGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    early_return_option_ready!(this.base_color_texture, cx);
    early_return_option_ready!(this.metallic_roughness_texture, cx);
    early_return_option_ready!(this.emissive_texture, cx);
    early_return_option_ready!(this.normal_texture, cx);
    Poll::Pending
  }
}

impl ShaderPassBuilder for PhysicalMetallicRoughnessMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform, SB::Material);
    if let Some(t) = self.base_color_texture.as_ref() {
      t.setup_pass(ctx, SB::Material)
    }
    if let Some(t) = self.metallic_roughness_texture.as_ref() {
      t.setup_pass(ctx, SB::Material)
    }
    if let Some(t) = self.emissive_texture.as_ref() {
      t.setup_pass(ctx, SB::Material)
    }
    if let Some(t) = self.normal_texture.as_ref() {
      t.setup_pass(ctx, SB::Material)
    }
  }
}

impl ShaderGraphProvider for PhysicalMetallicRoughnessMaterialGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.context.insert(
      ShadingSelection.type_id(),
      Box::new(&PhysicalShading as &dyn LightableSurfaceShadingDyn),
    );

    builder.fragment(|builder, binding| {
      let uniform = binding.uniform_by(&self.uniform, SB::Material).expand();
      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();

      let mut alpha = uniform.alpha;

      let base_color = if let Some(tex) = &self.base_color_texture {
        let sample = tex.uniform_and_sample(binding, SB::Material, uv);
        alpha *= sample.w();
        sample.xyz() * uniform.base_color
      } else {
        uniform.base_color
      };

      let mut metallic = uniform.metallic;
      let mut roughness = uniform.roughness;

      if let Some(tex) = &self.metallic_roughness_texture {
        let metallic_roughness = tex.uniform_and_sample(binding, SB::Material, uv);
        metallic *= metallic_roughness.x();
        roughness *= metallic_roughness.y();
      }

      let emissive = if let Some(tex) = &self.emissive_texture {
        tex.uniform_and_sample(binding, SB::Material, uv).x() * uniform.emissive
      } else {
        uniform.emissive
      };

      if let Some(tex) = &self.normal_texture {
        let normal_sample = tex.uniform_and_sample(binding, SB::Material, uv).xyz();
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

      builder.register::<ColorChannel>(base_color);
      builder.register::<EmissiveChannel>(emissive);
      builder.register::<MetallicChannel>(metallic);
      builder.register::<RoughnessChannel>(roughness);

      builder.register::<DefaultDisplay>((base_color, 1.));
      Ok(())
    })
  }
}

impl ReactiveRenderComponentSource for PhysicalMetallicRoughnessMaterialReactiveGPU {
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent {
    self.as_ref() as &dyn ReactiveRenderComponent
  }
}

type PhysicalMetallicRoughnessMaterialReactiveGPU = impl AsRef<RenderComponentCell<PhysicalMetallicRoughnessMaterialGPU>>
  + Stream<Item = RenderComponentDeltaFlag>;

use PhysicalMetallicRoughnessMaterialDelta as PD;

impl WebGPUMaterial for PhysicalMetallicRoughnessMaterial {
  type ReactiveGPU = PhysicalMetallicRoughnessMaterialReactiveGPU;
  fn create_reactive_gpu(
    source: &SceneItemRef<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU {
    let m = source.read();

    let uniform = build_shader_uniform(&m);
    let uniform = create_uniform2(uniform, &ctx.gpu.device);

    let base_color_texture = m
      .base_color_texture
      .as_ref()
      .map(|t| ctx.build_reactive_texture_sampler_pair(t));

    let metallic_roughness_texture = m
      .metallic_roughness_texture
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

    let gpu = PhysicalMetallicRoughnessMaterialGPU {
      uniform,
      base_color_texture,
      metallic_roughness_texture,
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
            state.uniform.resource.set(build_shader_uniform(&m.read()));
            state.uniform.resource.upload(&ctx.gpu.queue)
          }
          RenderComponentDeltaFlag::ContentRef
        }
        UniformChangePicked::Origin(delta) => match delta {
          PD::alpha_mode(_) => RenderComponentDeltaFlag::ShaderHash,
          PD::base_color_texture(t) => apply_tex_pair_delta(t, &mut state.base_color_texture, &ctx),
          PD::metallic_roughness_texture(t) => {
            apply_tex_pair_delta(t, &mut state.metallic_roughness_texture, &ctx)
          }
          PD::emissive_texture(t) => apply_tex_pair_delta(t, &mut state.emissive_texture, &ctx),
          PD::normal_texture(t) => apply_normal_map_delta(t, &mut state.normal_texture, &ctx),
          _ => RenderComponentDeltaFlag::Content, // handled in uniform
        },
      },
    )
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
