use rendiation_shader_library::normal_mapping::apply_normal_mapping_conditional;
use PhysicalSpecularGlossinessMaterialDelta as PD;

use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq)]
pub struct PhysicalSpecularGlossinessMaterialUniform {
  pub albedo: Vec3<f32>,
  pub specular: Vec3<f32>,
  pub emissive: Vec3<f32>,
  pub glossiness: f32,
  pub normal_mapping_scale: f32,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}

pub fn physical_sg_material_uniforms(
  cx: ResourceGPUCtx,
  scope: impl ReactiveCollection<AllocIdx<PhysicalSpecularGlossinessMaterial>, ()>,
) -> impl ReactiveCollection<
  AllocIdx<PhysicalSpecularGlossinessMaterial>,
  UniformBufferDataView<PhysicalSpecularGlossinessMaterialUniform>,
> {
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

  storage_of::<PhysicalSpecularGlossinessMaterial>()
    .listen_all_instance_changed_set()
    .filter_by_keyset(scope)
    .collective_create_uniforms(cx, |m| {
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
    })
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct PhysicalSpecularGlossinessMaterialTextureHandlesUniform {
  pub albedo_texture: TextureSamplerHandlePair,
  pub specular_texture: TextureSamplerHandlePair,
  pub emissive_texture: TextureSamplerHandlePair,
  pub glossiness_texture: TextureSamplerHandlePair,
  pub normal_texture: TextureSamplerHandlePair,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PhysicalSpecularGlossinessMaterialTextureType {
  Albedo,
  Specular,
  Emissive,
  Glossiness,
  Normal,
}
use PhysicalSpecularGlossinessMaterialTextureType as TextureType;

impl Into<u8> for TextureType {
  fn into(self) -> u8 {
    self as u8
  }
}

impl MaterialReferenceTexture for PhysicalSpecularGlossinessMaterial {
  type TextureType = TextureType;
  type TextureUniform = PhysicalSpecularGlossinessMaterialTextureHandlesUniform;

  fn get_texture(&self, ty: Self::TextureType) -> Option<&SceneTexture2D> {
    match ty {
      TextureType::Albedo => self.emissive_texture.as_ref().map(|t| &t.texture),
      TextureType::Specular => self.specular_texture.as_ref().map(|t| &t.texture),
      TextureType::Glossiness => self.glossiness_texture.as_ref().map(|t| &t.texture),
      TextureType::Emissive => self.emissive_texture.as_ref().map(|t| &t.texture),
      TextureType::Normal => self.normal_texture.as_ref().map(|t| &t.content.texture),
    }
  }

  fn check_change(
    change: Self::Delta,
  ) -> ChangeReaction<(Self::TextureType, AllocIdx<SceneTexture2DType>)> {
    todo!()
  }

  fn expand_self(&self, change: &mut dyn Fn((Self::TextureType, AllocIdx<SceneTexture2DType>))) {
    todo!()
  }
  fn update_texture_uniform(ty: Self::TextureType, handle: u32, target: &mut Self::TextureUniform) {
    todo!()
  }
}

pub fn physical_sg_material_texture_handle_uniforms(
  cx: &ResourceGPUCtx,
  scope: impl ReactiveCollection<AllocIdx<PhysicalSpecularGlossinessMaterial>, ()>,
) -> impl ReactiveCollection<
  AllocIdx<PhysicalSpecularGlossinessMaterial>,
  PhysicalSpecularGlossinessMaterialTextureHandlesUniform,
> {
  // tex_sample_handle_of_material().zip(..).zip(..).map(..)
}

pub struct PhysicalSpecularGlossinessMaterialGPU<'a> {
  uniform: &'a UniformBufferDataView<PhysicalSpecularGlossinessMaterialUniform>,
  tex_uniform: &'a UniformBufferDataView<PhysicalSpecularGlossinessMaterialTextureHandlesUniform>,
  source: &'a PhysicalSpecularGlossinessMaterial,
  binding_sys: &'a GPUTextureBindingSystem,
}

impl<'a> Deref for PhysicalSpecularGlossinessMaterialGPU<'a> {
  type Target = PhysicalSpecularGlossinessMaterial;

  fn deref(&self) -> &Self::Target {
    self.source
  }
}

impl<'a> ShaderHashProvider for PhysicalSpecularGlossinessMaterialGPU<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
}

impl<'a> ShaderPassBuilder for PhysicalSpecularGlossinessMaterialGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
    ctx.binding.bind(self.tex_uniform);
    setup_tex(ctx, self.binding_sys, &self.albedo_texture);
    setup_tex(ctx, self.binding_sys, &self.specular_texture);
    setup_tex(ctx, self.binding_sys, &self.glossiness_texture);
    setup_tex(ctx, self.binding_sys, &self.emissive_texture);
    setup_normal_tex(ctx, self.binding_sys, &self.normal_texture);
  }
}

impl<'a> GraphicsShaderProvider for PhysicalSpecularGlossinessMaterialGPU<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.context.insert(
      ShadingSelection.type_id(),
      Box::new(&PhysicalShading as &dyn LightableSurfaceShadingDyn),
    );

    builder.fragment(|builder, binding| {
      let uniform = binding.bind_by(&self.uniform).load().expand();
      let tex_uniform = binding.bind_by(&self.tex_uniform).load().expand();
      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();

      let mut alpha = uniform.alpha;

      let mut albedo = uniform.albedo;
      let albedo_tex = bind_and_sample(
        binding,
        builder.registry(),
        &self.albedo_texture,
        tex_uniform.albedo_texture,
        uv,
        val(Vec4::one()),
      );
      alpha *= albedo_tex.w();
      albedo *= albedo_tex.xyz();

      let mut specular = uniform.specular;
      specular *= bind_and_sample(
        binding,
        builder.registry(),
        &self.specular_texture,
        tex_uniform.specular_texture,
        uv,
        val(Vec4::one()),
      )
      .xyz();

      let mut glossiness = uniform.glossiness;
      glossiness *= bind_and_sample(
        binding,
        builder.registry(),
        &self.specular_texture,
        tex_uniform.glossiness_texture,
        uv,
        val(Vec4::one()),
      )
      .x();

      let mut emissive = uniform.emissive;
      emissive *= bind_and_sample(
        binding,
        builder.registry(),
        &self.emissive_texture,
        tex_uniform.emissive_texture,
        uv,
        val(Vec4::one()),
      )
      .xyz();

      let (normal_sample, enabled) = bind_and_sample_enabled(
        binding,
        builder.registry(),
        self.normal_texture.as_ref().map(|m| &m.content),
        tex_uniform.normal_texture,
        uv,
        val(Vec4::one()),
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

      builder.register::<ColorChannel>(albedo);
      builder.register::<SpecularChannel>(specular);
      builder.register::<EmissiveChannel>(emissive);
      builder.register::<GlossinessChannel>(glossiness);

      builder.register::<DefaultDisplay>((albedo, val(1.)));
      Ok(())
    })
  }
}
