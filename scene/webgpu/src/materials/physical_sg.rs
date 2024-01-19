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
  cx: &ResourceGPUCtx,
  scope: impl ReactiveCollection<AllocIdx<PhysicalSpecularGlossinessMaterial>, ()>,
) -> impl ReactiveCollection<
  AllocIdx<PhysicalSpecularGlossinessMaterial>,
  PhysicalSpecularGlossinessMaterialUniform,
> {
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

  let cx = cx.clone();
  storage_of::<PhysicalSpecularGlossinessMaterial>()
    .listen_all_instance_changed_set()
    .filter_by_keyset(scope)
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      let creator =
        storage_of::<PhysicalSpecularGlossinessMaterial>().create_key_mapper(move |m, _| {
          let cx = cx.clone();

          todo!()
        });
      move |k, _| creator(*k)
    })
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq)]
pub struct PhysicalSpecularGlossinessMaterialTextureHandlesUniform {
  pub albedo_texture: TextureSamplerHandlePair,
  pub specular_texture: TextureSamplerHandlePair,
  pub emissive_texture: TextureSamplerHandlePair,
  pub glossiness_texture: TextureSamplerHandlePair,
  pub normal_texture: TextureSamplerHandlePair,
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

#[pin_project::pin_project]
pub struct PhysicalSpecularGlossinessMaterialGPU<'a> {
  uniform: &'a UniformBufferDataView<PhysicalSpecularGlossinessMaterialUniform>,
  source: &'a PhysicalSpecularGlossinessMaterial,
  // textures: &'a TextureGetter,
}

impl<'a> ShaderHashProvider for PhysicalSpecularGlossinessMaterialGPU<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
}

impl<'a> ShaderPassBuilder for PhysicalSpecularGlossinessMaterialGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform);
    self.albedo_texture.setup_pass(ctx);
    self.specular_texture.setup_pass(ctx);
    self.glossiness_texture.setup_pass(ctx);
    self.emissive_texture.setup_pass(ctx);
    self.normal_texture.setup_pass(ctx);
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
      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();

      let mut alpha = uniform.alpha;

      let mut albedo = uniform.albedo;
      let albedo_tex = self.albedo_texture.bind_and_sample(
        binding,
        builder.registry(),
        uniform.albedo_texture,
        uv,
      );
      alpha *= albedo_tex.w();
      albedo *= albedo_tex.xyz();

      let mut specular = uniform.specular;
      specular *= self
        .specular_texture
        .bind_and_sample(binding, builder.registry(), uniform.specular_texture, uv)
        .xyz();

      let mut glossiness = uniform.glossiness;
      glossiness *= self
        .specular_texture
        .bind_and_sample(binding, builder.registry(), uniform.glossiness_texture, uv)
        .x();

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

      builder.register::<ColorChannel>(albedo);
      builder.register::<SpecularChannel>(specular);
      builder.register::<EmissiveChannel>(emissive);
      builder.register::<GlossinessChannel>(glossiness);

      builder.register::<DefaultDisplay>((albedo, val(1.)));
      Ok(())
    })
  }
}
