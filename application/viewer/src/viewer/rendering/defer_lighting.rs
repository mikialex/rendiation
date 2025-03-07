use std::any::TypeId;

use crate::*;

pub struct FrameGeneralMaterialBuffer {
  /// the following channel will be encode/decode by the different material type.
  pub material_type_id: RenderTargetView,
  pub channel_a: RenderTargetView,
  pub channel_b: RenderTargetView,
  pub channel_c: RenderTargetView,
}

impl FrameGeneralMaterialBuffer {
  pub fn new(cx: &mut FrameCtx) -> Self {
    Self {
      material_type_id: attachment().format(TextureFormat::R8Uint).request(cx),
      channel_a: attachment()
        .format(TextureFormat::Rgba8UnormSrgb)
        .request(cx),
      channel_b: attachment()
        .format(TextureFormat::Rgba8UnormSrgb)
        .request(cx),
      channel_c: attachment().format(TextureFormat::Rg16Float).request(cx),
    }
  }

  pub fn extend_pass_desc(
    &mut self,
    desc: &mut RenderPassDescription,
  ) -> FrameGeneralMaterialChannelIndices {
    FrameGeneralMaterialChannelIndices {
      material_type_id: desc.push_color(&self.material_type_id, clear(MAX_U8_ID_BACKGROUND)),
      channel_a: desc.push_color(&self.channel_a, clear(all_zero())),
      channel_b: desc.push_color(&self.channel_b, clear(all_zero())),
      channel_c: desc.push_color(&self.channel_c, clear(all_zero())),
    }
  }
}

pub const MAX_U8_ID_BACKGROUND: rendiation_webgpu::Color = rendiation_webgpu::Color {
  r: u8::MAX as f64,
  g: 0.,
  b: 0.,
  a: 0.,
};

pub struct FrameGeneralMaterialBufferReadValue {
  pub channel_a: Node<Vec4<f32>>,
  pub channel_b: Node<Vec4<f32>>,
  pub channel_c: Node<Vec2<f32>>,
}

#[derive(Hash)]
pub struct FrameGeneralMaterialChannelIndices {
  pub material_type_id: usize,
  pub channel_a: usize,
  pub channel_b: usize,
  pub channel_c: usize,
}

pub struct FrameGeneralMaterialBufferEncoder<'a> {
  pub indices: FrameGeneralMaterialChannelIndices,
  pub materials: &'a DeferLightingMaterialRegistry,
}

#[derive(Default, Clone)]
pub struct DeferLightingMaterialRegistry {
  pub material_impl_ids: Vec<TypeId>,
  pub encoders: Vec<fn(&mut ShaderFragmentBuilderView, &FrameGeneralMaterialChannelIndices)>,
  pub decoders: Vec<fn(&FrameGeneralMaterialBufferReadValue) -> Box<dyn LightableSurfaceShading>>,
}

pub trait DeferLightingMaterialBufferReadWrite: 'static {
  fn encode(builder: &mut ShaderFragmentBuilderView, indices: &FrameGeneralMaterialChannelIndices);
  fn decode(instance: &FrameGeneralMaterialBufferReadValue) -> Box<dyn LightableSurfaceShading>;
}

impl DeferLightingMaterialRegistry {
  pub fn register_material_impl<M: DeferLightingMaterialBufferReadWrite>(mut self) -> Self {
    self.material_impl_ids.push(TypeId::of::<M>());
    self.encoders.push(M::encode);
    self.decoders.push(M::decode);
    self
  }
}

impl ShaderHashProvider for FrameGeneralMaterialBufferEncoder<'_> {
  shader_hash_type_id! { FrameGeneralMaterialBufferEncoder<'static> }
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.indices.hash(hasher);
    self.materials.material_impl_ids.hash(hasher);
  }
}

impl ShaderPassBuilder for FrameGeneralMaterialBufferEncoder<'_> {}

impl GraphicsShaderProvider for FrameGeneralMaterialBufferEncoder<'_> {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      for (i, m) in self.materials.encoders.iter().enumerate() {
        m(builder, &self.indices);
        builder.frag_output[self.indices.material_type_id].store(val(i as u32));
      }
    })
  }
}

pub struct FrameGeneralMaterialBufferReconstructSurface<'a> {
  pub m_buffer: &'a FrameGeneralMaterialBuffer,
  pub registry: &'a DeferLightingMaterialRegistry,
}

impl ShaderHashProvider for FrameGeneralMaterialBufferReconstructSurface<'_> {
  shader_hash_type_id! { FrameGeneralMaterialBufferReconstructSurface<'static> }
}
impl ShaderPassBuilder for FrameGeneralMaterialBufferReconstructSurface<'_> {
  fn setup_pass(&self, cx: &mut GPURenderPassCtx) {
    self.m_buffer.material_type_id.bind_pass(cx);
    self.m_buffer.channel_a.bind_pass(cx);
    self.m_buffer.channel_b.bind_pass(cx);
    self.m_buffer.channel_c.bind_pass(cx);
    cx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
  }
}
impl LightableSurfaceProvider for FrameGeneralMaterialBufferReconstructSurface<'_> {
  fn construct_shading(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn LightableSurfaceShading> {
    let ids = binding.bind_by(&U32Texture2d);
    let channel_a = binding.bind_by(&self.m_buffer.channel_a);
    let channel_b = binding.bind_by(&self.m_buffer.channel_b);
    let channel_c = binding.bind_by(&self.m_buffer.channel_c);
    let sampler = binding.bind_by(&DisableFiltering(ImmediateGPUSamplerViewBind));

    let uv = builder.query::<FragmentUv>();

    let input_size = channel_a.texture_dimension_2d(None).into_f32();
    let u32_uv = (input_size * uv).floor().into_u32();
    let material_ty_id = ids.load_texel(u32_uv, val(0)).x();

    // discard compute to display the  background
    if_by(material_ty_id.equals(u8::MAX as u32), || {
      builder.discard();
    });

    let values = FrameGeneralMaterialBufferReadValue {
      channel_a: channel_a.sample_zero_level(sampler, uv),
      channel_b: channel_b.sample_zero_level(sampler, uv),
      channel_c: channel_c.sample_zero_level(sampler, uv).xy(),
    };

    Box::new(MultiMaterialUberDecoder {
      registry: self.registry.clone(),
      material_ty_id,
      data: values,
    })
  }
}

struct MultiMaterialUberDecoder {
  registry: DeferLightingMaterialRegistry,
  material_ty_id: Node<u32>,
  data: FrameGeneralMaterialBufferReadValue,
}

impl LightableSurfaceShading for MultiMaterialUberDecoder {
  fn compute_lighting_by_incident(
    &self,
    direct_light: &ENode<ShaderIncidentLight>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let diffuse = val(Vec3::<f32>::zero()).make_local_var();
    let specular = val(Vec3::<f32>::zero()).make_local_var();

    let mut switch = switch_by(self.material_ty_id);

    for (i, logic) in self.registry.decoders.iter().enumerate() {
      switch = switch.case(i as u32, || {
        let shading = logic(&self.data);
        let r = shading.compute_lighting_by_incident(direct_light, ctx);
        diffuse.store(r.diffuse);
        specular.store(r.specular);
      })
    }

    switch.end_with_default(|| {});

    ENode::<ShaderLightingResult> {
      diffuse: diffuse.load(),
      specular: specular.load(),
    }
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }
}

pub struct PbrSurfaceEncodeDecode;
// alpha is not used because in defer mode transparency is not supported
// and alpha discard has already done in encoder.
impl DeferLightingMaterialBufferReadWrite for PbrSurfaceEncodeDecode {
  fn encode(builder: &mut ShaderFragmentBuilderView, indices: &FrameGeneralMaterialChannelIndices) {
    if builder.contains_type_tag::<PbrMRMaterialTag>()
      || builder.contains_type_tag::<PbrSGMaterialTag>()
    {
      let ENode::<ShaderPhysicalShading> {
        albedo,
        linear_roughness,
        f0,
        emissive,
      } = PhysicalShading::construct_shading_impl(builder.registry());

      let albedo_roughness: Node<Vec4<_>> = (albedo, linear_roughness).into();
      let f0_emissive_x: Node<Vec4<_>> = (f0, emissive.x()).into();

      builder.frag_output[indices.channel_a].store(albedo_roughness);
      builder.frag_output[indices.channel_b].store(f0_emissive_x);
      builder.frag_output[indices.channel_c].store(emissive.yz());
    }
  }

  fn decode(instance: &FrameGeneralMaterialBufferReadValue) -> Box<dyn LightableSurfaceShading> {
    let albedo_roughness = instance.channel_a;
    let f0_emissive_x = instance.channel_b;
    let emissive_yz = instance.channel_c;

    let emissive = vec3_node((f0_emissive_x.w(), emissive_yz.x(), emissive_yz.y()));

    Box::new(ENode::<ShaderPhysicalShading> {
      albedo: albedo_roughness.xyz(),
      linear_roughness: albedo_roughness.w(),
      f0: f0_emissive_x.xyz(),
      emissive,
    })
  }
}
