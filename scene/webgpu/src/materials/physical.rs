use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct PhysicalMaterialUniform {
  pub albedo: Vec3<f32>,
  pub specular: Vec3<f32>,
  pub glossiness: f32,
}

impl ShaderHashProvider for PhysicalMaterialGPU {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    // todo optimize for reduce shader permutation
    self.albedo_texture.is_some().hash(hasher);
    self.specular_texture.is_some().hash(hasher);
    self.glossiness_texture.is_some().hash(hasher);
  }
}

pub struct PhysicalMaterialGPU {
  uniform: UniformBufferDataView<PhysicalMaterialUniform>,
  albedo_texture: Option<(GPU2DTextureView, GPUSamplerView)>,
  specular_texture: Option<(GPU2DTextureView, GPUSamplerView)>,
  glossiness_texture: Option<(GPU2DTextureView, GPUSamplerView)>,
}

impl ShaderPassBuilder for PhysicalMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform, SB::Material);
    if let Some(albedo_texture) = &self.albedo_texture {
      ctx.binding.bind(&albedo_texture.1, SB::Material);
      ctx.binding.bind(&albedo_texture.0, SB::Material);
    }
  }
}

impl ShaderGraphProvider for PhysicalMaterialGPU {
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

      let albedo = if let Some(albedo_texture) = &self.albedo_texture {
        let sampler = binding.uniform_by(&albedo_texture.1, SB::Material);
        let albedo_tex = binding.uniform_by(&albedo_texture.0, SB::Material);
        albedo_tex.sample(sampler, uv).xyz() * uniform.albedo
      } else {
        uniform.albedo
      };

      builder.register::<ColorChannel>(albedo);
      builder.register::<SpecularChannel>(consts(Vec3::splat(0.1)));
      builder.register::<RoughnessChannel>(consts(0.3));

      builder.register::<DefaultDisplay>((albedo, 1.));
      Ok(())
    })
  }
}

fn build_texture_sampler_pair<S>(
  t: &Texture2DWithSamplingData<S>,
  gpu: &GPU,
  res: &mut GPUResourceSubCache,
) -> (GPU2DTextureView, GPUSamplerView)
where
  S: SceneContent,
  S::Texture2D: AsRef<dyn WebGPU2DTextureSource>,
{
  let sampler = GPUSampler::create(t.sampler.into(), &gpu.device);
  let sampler = sampler.create_default_view();
  let tex = check_update_gpu_2d::<S>(&t.texture, res, gpu).clone();
  (tex, sampler)
}

impl<S> WebGPUMaterial for PhysicalMaterial<S>
where
  S: SceneContent,
  S::Texture2D: AsRef<dyn WebGPU2DTextureSource>,
{
  type GPU = PhysicalMaterialGPU;

  fn create_gpu(&self, res: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU {
    let uniform = PhysicalMaterialUniform {
      albedo: self.albedo,
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

    PhysicalMaterialGPU {
      uniform,
      albedo_texture,
      specular_texture,
      glossiness_texture,
    }
  }
  fn is_keep_mesh_shape(&self) -> bool {
    true
  }
  fn is_transparent(&self) -> bool {
    false
  }
}
