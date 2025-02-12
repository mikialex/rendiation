use std::any::TypeId;

use crate::*;

pub struct FrameGeneralMaterialBuffer {
  /// the following channel will be encode/decode by the different material type.
  pub material_type_id: Attachment,
  pub channel_a: Attachment,
  pub channel_b: Attachment,
  pub channel_c: Attachment,
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

  pub fn extend_pass_desc<'a>(
    &'a mut self,
    desc: &mut PassDescriptor<'a>,
  ) -> FrameGeneralMaterialChannelIndices {
    FrameGeneralMaterialChannelIndices {
      material_type_id: desc.push_color(self.material_type_id.write(), clear(all_zero())),
      channel_a: desc.push_color(self.channel_a.write(), clear(all_zero())),
      channel_b: desc.push_color(self.channel_b.write(), clear(all_zero())),
      channel_c: desc.push_color(self.channel_c.write(), clear(all_zero())),
    }
  }
}

pub struct FrameGeneralMaterialBufferShaderInstance {
  pub channel_a: HandleNode<ShaderTexture2D>,
  pub channel_b: HandleNode<ShaderTexture2D>,
  pub channel_c: HandleNode<ShaderTexture2D>,
  pub sampler: HandleNode<ShaderSampler>,
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
  pub decoders:
    Vec<fn(&FrameGeneralMaterialBufferShaderInstance) -> Box<dyn LightableSurfaceShading>>,
}

pub trait DeferLightingMaterialBufferReadWrite: 'static {
  fn encode(builder: &mut ShaderFragmentBuilderView, indices: &FrameGeneralMaterialChannelIndices);
  fn decode(
    instance: &FrameGeneralMaterialBufferShaderInstance,
  ) -> Box<dyn LightableSurfaceShading>;
}

impl DeferLightingMaterialRegistry {
  pub fn register_material_impl<M: DeferLightingMaterialBufferReadWrite>(&mut self) {
    self.material_impl_ids.push(TypeId::of::<M>());
    self.encoders.push(M::encode);
    self.decoders.push(M::decode);
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
      for m in &self.materials.encoders {
        m(builder, &self.indices);
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
    self.m_buffer.material_type_id.read().bind_pass(cx);
    self.m_buffer.channel_a.read().bind_pass(cx);
    self.m_buffer.channel_b.read().bind_pass(cx);
    self.m_buffer.channel_c.read().bind_pass(cx);
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
    let channel_a = binding.bind_by(&self.m_buffer.channel_a.read());
    let channel_b = binding.bind_by(&self.m_buffer.channel_b.read());
    let channel_c = binding.bind_by(&self.m_buffer.channel_c.read());
    let sampler = binding.bind_by(&DisableFiltering(ImmediateGPUSamplerViewBind));

    let uv = builder.query::<FragmentUv>();

    let input_size = channel_a.texture_dimension_2d(None).into_f32();
    let u32_uv = (input_size * uv).floor().into_u32();
    let material_ty_id = ids.load_texel(u32_uv, val(0)).x();

    Box::new(MultiMaterialUberDecoder {
      registry: self.registry.clone(),
      material_ty_id,
      data: FrameGeneralMaterialBufferShaderInstance {
        channel_a,
        channel_b,
        channel_c,
        sampler,
      },
    })
  }
}

struct MultiMaterialUberDecoder {
  registry: DeferLightingMaterialRegistry,
  material_ty_id: Node<u32>,
  data: FrameGeneralMaterialBufferShaderInstance,
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
