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

pub type ReactivePhysicalMetallicRoughnessMaterialGPU =
  impl Stream<Item = GPUResourceChange> + Unpin + AsRef<PhysicalMetallicRoughnessMaterialGPU>;

pub fn create_physical_metallic_material_gpu(
  material: SceneItemRef<PhysicalMetallicRoughnessMaterial>,
  res: &mut GPUResourceSubCache,
  gpu: &GPU,
) -> ReactivePhysicalMetallicRoughnessMaterialGPU {
  let gpu_material = material.read().create_gpu(res, gpu);
  material.listen_by(all_delta).fold_signal(
    gpu_material,
    |_, _: &mut PhysicalMetallicRoughnessMaterialGPU| {
      //
      GPUResourceChange::Reference
    },
  )
}

pub enum MaterialGPUReactive {
  PhysicalMetallicRoughnessMaterialGPU(ReactivePhysicalMetallicRoughnessMaterialGPU),
}

impl MaterialGPUReactive {
  pub fn as_render_component(&self) -> &dyn RenderComponent {
    match self {
      MaterialGPUReactive::PhysicalMetallicRoughnessMaterialGPU(gpu) => {
        gpu.as_ref() as &dyn RenderComponent
      }
    }
  }
}

impl WebGPUMaterial for PhysicalMetallicRoughnessMaterial {
  type GPU = PhysicalMetallicRoughnessMaterialGPU;

  fn create_gpu(&self, res: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU {
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
      .map(|t| build_texture_sampler_pair(t, gpu, res));

    let metallic_roughness_texture = self
      .metallic_roughness_texture
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
