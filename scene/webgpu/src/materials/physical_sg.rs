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

pub struct PhysicalSpecularGlossinessMaterialGPU {
  uniform: UniformBufferDataView<PhysicalSpecularGlossinessMaterialUniform>,
  albedo_texture: Option<GPUTextureSamplerPair>,
  specular_texture: Option<GPUTextureSamplerPair>,
  glossiness_texture: Option<GPUTextureSamplerPair>,
  emissive_texture: Option<GPUTextureSamplerPair>,
  normal_texture: Option<GPUTextureSamplerPair>,
  alpha_mode: AlphaMode,
}

impl ShaderPassBuilder for PhysicalSpecularGlossinessMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform, SB::Material);
    if let Some(t) = self.albedo_texture.as_ref() {
      t.setup_pass(ctx, SB::Material)
    }
    if let Some(t) = self.specular_texture.as_ref() {
      t.setup_pass(ctx, SB::Material)
    }
    if let Some(t) = self.glossiness_texture.as_ref() {
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
      let uniform = binding.uniform_by(&self.uniform, SB::Material).expand();
      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();

      let mut alpha = uniform.alpha;

      let albedo = if let Some(tex) = &self.albedo_texture {
        let sample = tex.uniform_and_sample(binding, SB::Material, uv);
        alpha *= sample.w();
        sample.xyz() * uniform.albedo
      } else {
        uniform.albedo
      };

      let specular = if let Some(tex) = &self.specular_texture {
        tex.uniform_and_sample(binding, SB::Material, uv).xyz() * uniform.specular
      } else {
        uniform.specular
      };

      let glossiness = if let Some(tex) = &self.glossiness_texture {
        tex.uniform_and_sample(binding, SB::Material, uv).x() * uniform.glossiness
      } else {
        uniform.glossiness
      };

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

      builder.register::<ColorChannel>(albedo);
      builder.register::<SpecularChannel>(specular);
      builder.register::<EmissiveChannel>(emissive);
      builder.register::<GlossinessChannel>(glossiness);

      builder.register::<DefaultDisplay>((albedo, 1.));
      Ok(())
    })
  }
}

impl WebGPUMaterial for PhysicalSpecularGlossinessMaterial {
  type GPU = PhysicalSpecularGlossinessMaterialGPU;

  fn create_gpu(&self, res: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU {
    let mut uniform = PhysicalSpecularGlossinessMaterialUniform {
      albedo: self.albedo,
      specular: self.specular,
      emissive: self.emissive,
      glossiness: self.glossiness,
      normal_mapping_scale: 1.,
      alpha_cutoff: self.alpha_cutoff,
      alpha: self.alpha,
      ..Zeroable::zeroed()
    };

    let albedo_texture = self
      .albedo_texture
      .as_ref()
      .map(|t| build_texture_sampler_pair(t, gpu, res));

    let glossiness_texture = self
      .glossiness_texture
      .as_ref()
      .map(|t| build_texture_sampler_pair(t, gpu, res));

    let specular_texture = self
      .specular_texture
      .as_ref()
      .map(|t| build_texture_sampler_pair(t, gpu, res));

    let emissive_texture = self
      .emissive_texture
      .as_ref()
      .map(|t| build_texture_sampler_pair(t, gpu, res));

    let normal_texture = self.normal_texture.as_ref().map(|t| {
      uniform.normal_mapping_scale = t.scale;
      build_texture_sampler_pair(&t.content, gpu, res)
    });

    let uniform = create_uniform(uniform, gpu);

    PhysicalSpecularGlossinessMaterialGPU {
      uniform,
      albedo_texture,
      specular_texture,
      glossiness_texture,
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
