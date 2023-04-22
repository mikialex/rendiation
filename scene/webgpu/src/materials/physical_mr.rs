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

pub struct PhysicalMetallicRoughnessMaterialGPU {
  uniform: UniformBufferDataView<PhysicalMetallicRoughnessMaterialUniform>,
  base_color_texture: Option<GPUTextureSamplerPair>,
  metallic_roughness_texture: Option<GPUTextureSamplerPair>,
  emissive_texture: Option<GPUTextureSamplerPair>,
  normal_texture: Option<GPUTextureSamplerPair>,
  alpha_mode: AlphaMode,
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

impl WebGPUMaterial for PhysicalMetallicRoughnessMaterial {
  type GPU = PhysicalMetallicRoughnessMaterialGPU;

  fn create_gpu(&self, res: &mut ShareBindableResourceCtx, gpu: &GPU) -> Self::GPU {
    let mut uniform = PhysicalMetallicRoughnessMaterialUniform {
      base_color: self.base_color,
      roughness: self.roughness,
      emissive: self.emissive,
      metallic: self.metallic,
      reflectance: self.reflectance,
      normal_mapping_scale: 1.,
      alpha_cutoff: self.alpha_cutoff,
      alpha: self.alpha,
      ..Zeroable::zeroed()
    };

    let base_color_texture = self
      .base_color_texture
      .as_ref()
      .map(|t| res.build_texture_sampler_pair(t));

    let metallic_roughness_texture = self
      .metallic_roughness_texture
      .as_ref()
      .map(|t| res.build_texture_sampler_pair(t));

    let emissive_texture = self
      .emissive_texture
      .as_ref()
      .map(|t| res.build_texture_sampler_pair(t));

    let normal_texture = self.normal_texture.as_ref().map(|t| {
      uniform.normal_mapping_scale = t.scale;
      res.build_texture_sampler_pair(&t.content)
    });

    let uniform = create_uniform(uniform, gpu);

    PhysicalMetallicRoughnessMaterialGPU {
      uniform,
      base_color_texture,
      metallic_roughness_texture,
      emissive_texture,
      normal_texture,
      alpha_mode: self.alpha_mode,
    }
  }
  fn is_keep_mesh_shape(&self) -> bool {
    true
  }
  fn is_transparent(&self) -> bool {
    matches!(self.alpha_mode, AlphaMode::Blend)
  }
}

type PhysicalMetallicRoughnessMaterialGPUReactiveInner = RenderComponentReactive<
  PhysicalMetallicRoughnessMaterialGPU,
  PhysicalMetallicRoughnessMaterialReactive,
>;

pub type PhysicalMetallicRoughnessMaterialGPUReactive = impl AsRef<RenderComponentCell<PhysicalMetallicRoughnessMaterialGPUReactiveInner>>
  + Stream<Item = RenderComponentDelta>;

fn build_shader_uniform(
  m: &PhysicalMetallicRoughnessMaterial,
) -> PhysicalMetallicRoughnessMaterialUniform {
  PhysicalMetallicRoughnessMaterialUniform {
    base_color: m.base_color,
    roughness: m.roughness,
    emissive: m.emissive,
    metallic: m.metallic,
    reflectance: m.reflectance,
    normal_mapping_scale: 1.,
    alpha_cutoff: m.alpha_cutoff,
    alpha: m.alpha,
    ..Zeroable::zeroed()
  }
}

pub fn physical_metallic_roughness_material_build_gpu(
  source: &SceneItemRef<PhysicalMetallicRoughnessMaterial>,
  ctx: &ShareBindableResourceCtx,
) -> PhysicalMetallicRoughnessMaterialGPUReactive {
  let m = source.read();

  let mut uniform = build_shader_uniform(&m);

  let base_color_texture = m
    .base_color_texture
    .as_ref()
    .map(|t| ctx.build_texture_sampler_pair(t));

  let metallic_roughness_texture = m
    .metallic_roughness_texture
    .as_ref()
    .map(|t| ctx.build_texture_sampler_pair(t));

  let emissive_texture = m
    .emissive_texture
    .as_ref()
    .map(|t| ctx.build_texture_sampler_pair(t));

  let normal_texture = m.normal_texture.as_ref().map(|t| {
    uniform.normal_mapping_scale = t.scale;
    ctx.build_texture_sampler_pair(&t.content)
  });

  let uniform = create_uniform2(uniform, &ctx.gpu.device);

  let gpu = PhysicalMetallicRoughnessMaterialGPU {
    uniform,
    base_color_texture,
    metallic_roughness_texture,
    emissive_texture,
    normal_texture,
    alpha_mode: m.alpha_mode,
  };

  let state = RenderComponentReactive::from_gpu_with_default_reactive(gpu);
  let state = RenderComponentCell::new(state);

  use PhysicalMetallicRoughnessMaterialDelta as PD;
  let ctx = ctx.clone();
  source.listen_by(all_delta).fold_signal_flatten(
    state,
    move |delta,
          state: &mut RenderComponentCell<PhysicalMetallicRoughnessMaterialGPUReactiveInner>| {
      match delta {
        PD::alpha_mode(_) => RenderComponentDelta::ShaderHash,
        PD::base_color_texture(t) => {
          // let (t, tx) = ctx.build_texture_sampler_pair(todo!());
          // state.gpu.reactive.base_color = tx;
          // state.gpu.gpu.base_color_texture = t;
          RenderComponentDelta::ContentRef
        }
        PD::metallic_roughness_texture(_) => todo!(),
        PD::emissive_texture(_) => todo!(),
        PD::normal_texture(_) => todo!(),
        _ => RenderComponentDelta::Content,
      }
    },
  )
}

#[pin_project::pin_project]
#[derive(Default)]
pub struct PhysicalMetallicRoughnessMaterialReactive {
  base_color: Option<Texture2dRenderComponentDeltaStream>,
  metallic_roughness: Option<Texture2dRenderComponentDeltaStream>,
  emissive: Option<Texture2dRenderComponentDeltaStream>,
  normal: Option<Texture2dRenderComponentDeltaStream>,
}

impl Stream for PhysicalMetallicRoughnessMaterialReactive {
  type Item = RenderComponentDelta;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    early_return_option_ready!(this.base_color, cx);
    early_return_option_ready!(this.metallic_roughness, cx);
    early_return_option_ready!(this.emissive, cx);
    early_return_option_ready!(this.normal, cx);
    Poll::Pending
  }
}
