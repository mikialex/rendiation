use crate::*;

pub fn default_dispatcher(pass: &FrameRenderPass, reversed_depth: bool) -> DefaultPassDispatcher {
  DefaultPassDispatcher {
    formats: pass.ctx.pass.formats().clone(),
    pass_info: pass.pass_info.clone(),
    auto_write: true,
    reversed_depth,
  }
}

pub struct DefaultPassDispatcher {
  pub formats: RenderTargetFormatsInfo,
  pub auto_write: bool,
  pub reversed_depth: bool,
  pub pass_info: UniformBufferCachedDataView<RenderPassGPUInfoData>,
}

impl ShaderHashProvider for DefaultPassDispatcher {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.formats.hash(hasher);
    self.auto_write.hash(hasher);
  }
  shader_hash_type_id! {}
}
impl ShaderPassBuilder for DefaultPassDispatcher {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.pass_info);
  }
}

impl GraphicsShaderProvider for DefaultPassDispatcher {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    let pass = builder.bind_by_and_prepare(&self.pass_info);

    builder.vertex(|builder, _| {
      let pass = pass.using().load().expand();
      builder.register::<RenderBufferSize>(pass.buffer_size);
      builder.register::<TexelSize>(pass.texel_size);
    });

    builder.fragment(|builder, _| {
      let pass = pass.using().load().expand();
      builder.register::<RenderBufferSize>(pass.buffer_size);
      builder.register::<TexelSize>(pass.texel_size);

      for &format in &self.formats.color_formats {
        builder.define_out_by(channel(format));
      }

      builder.depth_stencil = self
        .formats
        .depth_stencil_formats
        .map(|format| DepthStencilState {
          format,
          depth_write_enabled: true,
          depth_compare: if self.reversed_depth {
            CompareFunction::Greater
          } else {
            CompareFunction::Less
          },
          stencil: Default::default(),
          bias: Default::default(),
        });

      builder.multisample.count = self.formats.sample_count;
    })
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      if self.auto_write && !self.formats.color_formats.is_empty() {
        if let Some(first) = self.formats.color_formats.first() {
          if get_suitable_shader_write_ty_from_texture_format(*first).unwrap()
            == ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Vec4Float32)
          {
            let default = builder.query_or_insert_default::<DefaultDisplay>();
            builder.store_fragment_out(0, default)
          }
        }
      }
    })
  }
}
