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
}

impl ShaderHashProvider for PhysicalMetallicRoughnessMaterialGPU {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    // todo optimize for reduce shader permutation
    self.base_color_texture.is_some().hash(hasher);
    self.metallic_roughness_texture.is_some().hash(hasher);
    self.emissive_texture.is_some().hash(hasher);
  }
}

pub struct PhysicalMetallicRoughnessMaterialGPU {
  uniform: UniformBufferDataView<PhysicalMetallicRoughnessMaterialUniform>,
  base_color_texture: Option<GPUTextureSamplerPair>,
  metallic_roughness_texture: Option<GPUTextureSamplerPair>,
  emissive_texture: Option<GPUTextureSamplerPair>,
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

      let base_color = if let Some(tex) = &self.base_color_texture {
        tex.uniform_and_sample(binding, SB::Material, uv).xyz() * uniform.base_color
      } else {
        uniform.base_color
      };

      let mut metallic = uniform.metallic;
      let mut roughness = uniform.roughness;

      if let Some(tex) = &self.metallic_roughness_texture {
        let metallic_roughness = tex.uniform_and_sample(binding, SB::Material, uv);
        metallic = metallic * metallic_roughness.x();
        roughness = roughness * metallic_roughness.y();
      }

      let emissive = if let Some(tex) = &self.emissive_texture {
        tex.uniform_and_sample(binding, SB::Material, uv).x() * uniform.emissive
      } else {
        uniform.emissive
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

impl<S> WebGPUMaterial for PhysicalMetallicRoughnessMaterial<S>
where
  S: SceneContent,
  S::Texture2D: AsRef<dyn WebGPU2DTextureSource>,
{
  type GPU = PhysicalMetallicRoughnessMaterialGPU;

  fn create_gpu(&self, res: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU {
    let uniform = PhysicalMetallicRoughnessMaterialUniform {
      base_color: self.base_color,
      roughness: self.roughness,
      emissive: self.emissive,
      metallic: self.metallic,
      reflectance: self.reflectance,
      ..Zeroable::zeroed()
    };
    let uniform = create_uniform(uniform, gpu);

    let base_color_texture = self
      .base_color_texture
      .as_ref()
      .map(|t| build_texture_sampler_pair::<S>(t, gpu, res));

    let metallic_roughness_texture = self
      .metallic_roughness_texture
      .as_ref()
      .map(|t| build_texture_sampler_pair::<S>(t, gpu, res));

    let emissive_texture = self
      .emissive_texture
      .as_ref()
      .map(|t| build_texture_sampler_pair::<S>(t, gpu, res));

    PhysicalMetallicRoughnessMaterialGPU {
      uniform,
      base_color_texture,
      metallic_roughness_texture,
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
