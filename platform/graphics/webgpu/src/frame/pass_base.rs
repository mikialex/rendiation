use crate::*;

pub fn default_dispatcher(pass: &FrameRenderPass) -> DefaultPassDispatcher {
  DefaultPassDispatcher {
    formats: pass.ctx.pass.formats().clone(),
    pass_info: pass.pass_info.clone(),
    auto_write: true,
  }
}

pub struct DefaultPassDispatcher {
  pub formats: RenderTargetFormatsInfo,
  pub auto_write: bool,
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
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    let pass = builder.bind_by(&self.pass_info);

    builder.vertex(|builder, _| {
      let pass = pass.using().load().expand();
      builder.register::<RenderBufferSize>(pass.buffer_size);
      builder.register::<TexelSize>(pass.texel_size);
      Ok(())
    })?;

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
          depth_compare: CompareFunction::Less,
          stencil: Default::default(),
          bias: Default::default(),
        });

      builder.multisample.count = self.formats.sample_count;

      Ok(())
    })
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, _| {
      if self.auto_write && !self.formats.color_formats.is_empty() {
        let default = builder.query_or_insert_default::<DefaultDisplay>();
        builder.store_fragment_out(0, default)
      } else {
        Ok(())
      }
    })
  }
}
