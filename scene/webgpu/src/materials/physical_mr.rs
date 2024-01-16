use rendiation_shader_library::normal_mapping::apply_normal_mapping_conditional;
use PhysicalMetallicRoughnessMaterialDelta as PD;

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

pub fn physical_mr_material_uniforms(
  cx: &ResourceGPUCtx,
  scope: impl ReactiveCollection<AllocIdx<PhysicalMetallicRoughnessMaterial>, ()>,
) -> impl ReactiveCollection<
  AllocIdx<PhysicalMetallicRoughnessMaterial>,
  PhysicalMetallicRoughnessMaterialUniform,
> {
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

  let cx = cx.clone();
  storage_of::<PhysicalMetallicRoughnessMaterial>()
    .listen_to_reactive_collection(|_| Some(()))
    .filter_by_keyset(scope)
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      let creator = storage_of::<PhysicalMetallicRoughnessMaterial>().create_key_mapper(move |m| {
        let cx = cx.clone();

        todo!()
      });
      move |k, _| creator(*k)
    })
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct PhysicalMetallicRoughnessMaterialTextureHandlesUniform {
  pub base_color_texture: TextureSamplerHandlePair,
  pub emissive_texture: TextureSamplerHandlePair,
  pub metallic_roughness_texture: TextureSamplerHandlePair,
  pub normal_texture: TextureSamplerHandlePair,
}

pub fn physical_mr_material_texture_handle_uniforms(
  cx: &ResourceGPUCtx,
  scope: impl ReactiveCollection<AllocIdx<PhysicalMetallicRoughnessMaterial>, ()>,
) -> impl ReactiveCollection<
  AllocIdx<PhysicalMetallicRoughnessMaterial>,
  PhysicalMetallicRoughnessMaterialTextureHandlesUniform,
> {
  // tex_sample_handle_of_material().zip(..).zip(..).map(..)
}

pub struct PhysicalMetallicRoughnessMaterialGPU<'a> {
  uniform: &'a UniformBufferDataView<PhysicalMetallicRoughnessMaterialUniform>,
  source: &'a PhysicalMetallicRoughnessMaterial,
  // textures: &'a TextureGetter,
}

impl<'a> ShaderHashProvider for PhysicalMetallicRoughnessMaterialGPU<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.source.alpha_mode.hash(hasher);
  }
}

impl<'a> ShaderPassBuilder for PhysicalMetallicRoughnessMaterialGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform);
    self.base_color_texture.setup_pass(ctx);
    self.metallic_roughness_texture.setup_pass(ctx);
    self.emissive_texture.setup_pass(ctx);
    self.normal_texture.setup_pass(ctx);
  }
}

impl<'a> GraphicsShaderProvider for PhysicalMetallicRoughnessMaterialGPU<'a> {
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
