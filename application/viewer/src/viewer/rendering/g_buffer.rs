use crate::*;

pub struct FrameGeometryBuffer {
  pub depth: Attachment,
  pub normal: Attachment,
  pub entity_id: Attachment,
}

pub const ID_BACKGROUND: rendiation_webgpu::Color = rendiation_webgpu::Color {
  r: u32::MAX as f64,
  g: 0.,
  b: 0.,
  a: 0.,
};

impl FrameGeometryBuffer {
  pub fn new(cx: &mut FrameCtx) -> Self {
    Self {
      depth: depth_attachment().request(cx),
      normal: attachment().format(TextureFormat::Rgb10a2Unorm).request(cx),
      entity_id: attachment().format(TextureFormat::R32Uint).request(cx),
    }
  }
}

pub struct FrameGeometryBufferPassEncoder {
  pub normal: usize,
  pub entity_id: usize,
}

impl ShaderHashProvider for FrameGeometryBufferPassEncoder {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.normal.hash(hasher);
    self.entity_id.hash(hasher);
  }
}

impl ShaderPassBuilder for FrameGeometryBufferPassEncoder {}

impl GraphicsShaderProvider for FrameGeometryBufferPassEncoder {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let id = builder.query_or_interpolate_by::<LogicalRenderEntityId, LogicalRenderEntityId>();
      builder.frag_output[self.entity_id].store(id);

      let normal = builder.query_or_interpolate_by::<FragmentWorldNormal, WorldVertexNormal>();
      let out: Node<Vec4<f32>> = (normal, val(1.0)).into();
      builder.frag_output[self.normal].store(out);
    })
  }
}
