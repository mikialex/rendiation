use rendiation_shader_library::normal_mapping::apply_normal_mapping_conditional;
use PhysicalMetallicRoughnessMaterialDelta as PD;

use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq)]
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
  cx: ResourceGPUCtx,
  scope: impl ReactiveCollection<AllocIdx<PhysicalMetallicRoughnessMaterial>, ()>,
) -> impl ReactiveCollectionSelfContained<
  AllocIdx<PhysicalMetallicRoughnessMaterial>,
  UniformBufferDataView<PhysicalMetallicRoughnessMaterialUniform>,
> {
  fn is_uniform_changed(d: &DeltaOf<PhysicalMetallicRoughnessMaterial>) -> bool {
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

  storage_of::<PhysicalMetallicRoughnessMaterial>()
    .listen_to_reactive_collection(|delta| match delta {
      MaybeDeltaRef::Delta(d) => {
        if is_uniform_changed(d) {
          ChangeReaction::Care(Some(AnyChanging))
        } else {
          ChangeReaction::NotCare
        }
      }
      MaybeDeltaRef::All(_) => ChangeReaction::Care(Some(AnyChanging)),
    })
    .filter_by_keyset(scope)
    .collective_create_uniforms_by_key(cx, |m| {
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
    })
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct PhysicalMetallicRoughnessMaterialTextureHandlesUniform {
  pub base_color_texture: TextureSamplerHandlePair,
  pub emissive_texture: TextureSamplerHandlePair,
  pub metallic_roughness_texture: TextureSamplerHandlePair,
  pub normal_texture: TextureSamplerHandlePair,
}

use num_derive::*;
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, FromPrimitive, ToPrimitive)]
pub enum PhysicalMetallicRoughnessMaterialTextureType {
  BaseColor,
  MetallicRoughness,
  Emissive,
  Normal,
}
use PhysicalMetallicRoughnessMaterialTextureType as TextureType;

impl MaterialReferenceTexture for PhysicalMetallicRoughnessMaterial {
  type TextureType = TextureType;
  type TextureUniform = PhysicalMetallicRoughnessMaterialTextureHandlesUniform;

  fn get_texture(&self, ty: Self::TextureType) -> Option<&SceneTexture2D> {
    match ty {
      TextureType::BaseColor => pick_tex(&self.base_color_texture),
      TextureType::MetallicRoughness => pick_tex(&self.metallic_roughness_texture),
      TextureType::Emissive => pick_tex(&self.emissive_texture),
      TextureType::Normal => self.normal_texture.as_ref().map(|t| &t.content.texture),
    }
  }

  fn react_change(
    &self,
    delta: &Self::Delta,
    callback: &dyn Fn(Self::TextureType, Option<AllocIdx<SceneTexture2DType>>),
  ) {
    let (t, d) = match delta {
      PD::base_color_texture(t) => (TextureType::BaseColor, pick_tex_d(t)),
      PD::metallic_roughness_texture(t) => (TextureType::MetallicRoughness, pick_tex_d(t)),
      PD::emissive_texture(t) => (TextureType::Emissive, pick_tex_d(t)),
      PD::normal_texture(t) => (TextureType::Normal, pick_normal_tex_d(t)),
      _ => return,
    };
    callback(t, d)
  }

  fn create_iter(&self) -> impl Iterator<Item = (Self::TextureType, AllocIdx<SceneTexture2DType>)> {
    [
      pick_tex_id(&self.base_color_texture).map(|id| (TextureType::BaseColor, id)),
      pick_tex_id(&self.metallic_roughness_texture).map(|id| (TextureType::MetallicRoughness, id)),
      pick_tex_id(&self.emissive_texture).map(|id| (TextureType::Emissive, id)),
      // pick_tex_id(&self.normal_texture).map(|id| (TextureType::BaseColor, id)),
    ]
    .into_iter()
    .flatten()
  }

  fn update_texture_uniform(ty: Self::TextureType, handle: u32, target: &mut Self::TextureUniform) {
    match ty {
      TextureType::BaseColor => target.base_color_texture.texture_handle = handle,
      TextureType::MetallicRoughness => target.metallic_roughness_texture.texture_handle = handle,
      TextureType::Emissive => target.emissive_texture.texture_handle = handle,
      TextureType::Normal => target.normal_texture.texture_handle = handle,
    }
  }
}

pub struct PhysicalMetallicRoughnessMaterialGPU<'a> {
  uniform: &'a UniformBufferDataView<PhysicalMetallicRoughnessMaterialUniform>,
  texture_uniforms:
    &'a UniformBufferDataView<PhysicalMetallicRoughnessMaterialTextureHandlesUniform>,
  source: &'a PhysicalMetallicRoughnessMaterial,
  binding_sys: &'a GPUTextureBindingSystem,
}
impl<'a> Deref for PhysicalMetallicRoughnessMaterialGPU<'a> {
  type Target = PhysicalMetallicRoughnessMaterial;

  fn deref(&self) -> &Self::Target {
    self.source
  }
}

impl<'a> ShaderHashProvider for PhysicalMetallicRoughnessMaterialGPU<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
}

impl<'a> ShaderPassBuilder for PhysicalMetallicRoughnessMaterialGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
    ctx.binding.bind(self.texture_uniforms);
    setup_tex(ctx, self.binding_sys, &self.base_color_texture);
    setup_tex(ctx, self.binding_sys, &self.metallic_roughness_texture);
    setup_tex(ctx, self.binding_sys, &self.emissive_texture);
    setup_normal_tex(ctx, self.binding_sys, &self.normal_texture);
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
      let tex_uniform = binding.bind_by(&self.texture_uniforms).load().expand();

      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();

      let mut alpha = uniform.alpha;
      let mut base_color = uniform.base_color;

      let base_color_tex = bind_and_sample(
        binding,
        builder.registry(),
        &self.base_color_texture,
        tex_uniform.base_color_texture,
        uv,
        val(Vec4::one()),
      );
      alpha *= base_color_tex.w();
      base_color *= base_color_tex.xyz();

      let mut metallic = uniform.metallic;
      let mut roughness = uniform.roughness;

      let metallic_roughness_tex = bind_and_sample(
        binding,
        builder.registry(),
        &self.metallic_roughness_texture,
        tex_uniform.metallic_roughness_texture,
        uv,
        val(Vec4::one()),
      );

      metallic *= metallic_roughness_tex.x();
      roughness *= metallic_roughness_tex.y();

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

      builder.register::<ColorChannel>(base_color);
      builder.register::<EmissiveChannel>(emissive);
      builder.register::<MetallicChannel>(metallic);
      builder.register::<RoughnessChannel>(roughness * roughness);

      builder.register::<DefaultDisplay>((base_color, val(1.)));
      Ok(())
    })
  }
}
