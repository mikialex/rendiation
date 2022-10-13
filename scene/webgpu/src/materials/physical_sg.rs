use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct PhysicalSpecularGlossinessMaterialUniform {
  pub albedo: Vec3<f32>,
  pub specular: Vec3<f32>,
  pub emissive: Vec3<f32>,
  pub glossiness: f32,
}

impl ShaderHashProvider for PhysicalSpecularGlossinessMaterialGPU {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    // todo optimize for reduce shader permutation
    self.albedo_texture.is_some().hash(hasher);
    self.specular_texture.is_some().hash(hasher);
    self.glossiness_texture.is_some().hash(hasher);
    self.emissive_texture.is_some().hash(hasher);
  }
}

pub struct PhysicalSpecularGlossinessMaterialGPU {
  uniform: UniformBufferDataView<PhysicalSpecularGlossinessMaterialUniform>,
  albedo_texture: Option<GPUTextureSamplerPair>,
  specular_texture: Option<GPUTextureSamplerPair>,
  glossiness_texture: Option<GPUTextureSamplerPair>,
  emissive_texture: Option<GPUTextureSamplerPair>,
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

      let albedo = if let Some(tex) = &self.albedo_texture {
        tex.uniform_and_sample(binding, SB::Material, uv).xyz() * uniform.albedo
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

      builder.register::<ColorChannel>(albedo);
      builder.register::<SpecularChannel>(specular);
      builder.register::<EmissiveChannel>(emissive);
      builder.register::<GlossinessChannel>(glossiness);

      builder.register::<DefaultDisplay>((albedo, 1.));
      Ok(())
    })
  }
}

impl<S> WebGPUMaterial for PhysicalSpecularGlossinessMaterial<S>
where
  S: SceneContent,
  S::Texture2D: AsRef<dyn WebGPU2DTextureSource>,
{
  type GPU = PhysicalSpecularGlossinessMaterialGPU;

  fn create_gpu(&self, res: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU {
    let uniform = PhysicalSpecularGlossinessMaterialUniform {
      albedo: self.albedo,
      specular: self.specular,
      emissive: self.emissive,
      glossiness: self.glossiness,
      ..Zeroable::zeroed()
    };
    let uniform = create_uniform(uniform, gpu);

    let albedo_texture = self
      .albedo_texture
      .as_ref()
      .map(|t| build_texture_sampler_pair::<S>(t, gpu, res));

    let glossiness_texture = self
      .glossiness_texture
      .as_ref()
      .map(|t| build_texture_sampler_pair::<S>(t, gpu, res));

    let specular_texture = self
      .specular_texture
      .as_ref()
      .map(|t| build_texture_sampler_pair::<S>(t, gpu, res));

    let emissive_texture = self
      .emissive_texture
      .as_ref()
      .map(|t| build_texture_sampler_pair::<S>(t, gpu, res));

    PhysicalSpecularGlossinessMaterialGPU {
      uniform,
      albedo_texture,
      specular_texture,
      glossiness_texture,
      emissive_texture,
    }
  }
  fn is_keep_mesh_shape(&self) -> bool {
    true
  }
  fn is_transparent(&self) -> bool {
    false
  }
}
