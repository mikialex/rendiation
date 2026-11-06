use crate::*;

pub struct FrameRenderPass {
  pub ctx: GPURenderPassCtx,
  pub pass_info_pool: PassInfoPool,
  /// access pass_info_pool requires locking, so we keep here to reduce the locking to pass level
  pub pass_info: UniformBufferDataView<RenderPassGPUInfoData>,
  /// None means the default full attachment viewport
  viewport: Option<[f32; 6]>,
}

impl FrameRenderPass {
  pub fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32, min_depth: f32, max_depth: f32) {
    self.ctx.pass.set_viewport(x, y, w, h, min_depth, max_depth);
    self.pass_info = self
      .pass_info_pool
      .get_pass_info((w, h).into(), &self.ctx.gpu.device);
    self.viewport = Some([x, y, w, h, min_depth, max_depth]);
  }

  /// the viewport set by [FrameRenderPass::set_viewport], None means the default full
  /// attachment viewport.
  pub fn viewport(&self) -> Option<[f32; 6]> {
    self.viewport
  }

  /// restore the viewport returned by [FrameRenderPass::viewport]
  pub fn restore_viewport(&mut self, viewport: Option<[f32; 6]>) {
    if let Some([x, y, w, h, min_depth, max_depth]) = viewport {
      self.set_viewport(x, y, w, h, min_depth, max_depth);
    } else {
      let (w, h) = self.ctx.pass.size().into_f32();
      self.set_viewport(0., 0., w, h, 0., 1.);
      self.viewport = None;
    }
  }
}

impl std::ops::Deref for FrameRenderPass {
  type Target = GPURenderPass;

  fn deref(&self) -> &Self::Target {
    &self.ctx.pass
  }
}

impl std::ops::DerefMut for FrameRenderPass {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.ctx.pass
  }
}

pub trait RenderPassDescriptionFrameExt {
  fn render_ctx(self, ctx: &mut FrameCtx) -> ActiveRenderPass;

  /// the pass info pool will be created if not provided, in this case the pass info uniform
  /// buffer will not be reused.
  fn render(
    self,
    encoder: &mut GPUCommandEncoder,
    gpu: &GPU,
    statistics: Option<&mut FramePassStatisticsRecorder>,
    pass_info_pool: Option<PassInfoPool>,
  ) -> ActiveRenderPass;
}

impl RenderPassDescriptionFrameExt for RenderPassDescription {
  fn render_ctx(self, ctx: &mut FrameCtx) -> ActiveRenderPass {
    self.render(
      &mut ctx.encoder,
      ctx.gpu,
      ctx.statistics.as_mut(),
      Some(ctx.pass_info_pool.clone()),
    )
  }

  fn render(
    self,
    encoder: &mut GPUCommandEncoder,
    gpu: &GPU,
    statistics: Option<&mut FramePassStatisticsRecorder>,
    pass_info_pool: Option<PassInfoPool>,
  ) -> ActiveRenderPass {
    let pass_info_pool = pass_info_pool.unwrap_or_default();
    let pass_info = pass_info_pool.get_pass_info(self.buffer_size(), &gpu.device);

    let (pass, pipeline_query_active) = if let Some(statistics) = statistics {
      statistics.begin_pass(&self, encoder, &gpu.device)
    } else {
      (encoder.begin_render_pass(&self, None), false)
    };

    ActiveRenderPass {
      pass: FrameRenderPass {
        ctx: GPURenderPassCtx::new(pass, gpu.clone()),
        pass_info,
        pass_info_pool,
        viewport: None,
      },
      desc: self,
      pipeline_query_active,
    }
  }
}

pub trait PassContent {
  // the pass content will be scoped with this debug name
  // the implementation can override this name
  fn debug_label(&self) -> String {
    disqualified::ShortName::of::<Self>().to_string()
  }
  fn render_with_scope_label(&mut self, pass: &mut FrameRenderPass) {
    pass.push_debug_group(&self.debug_label());
    self.render(pass);
    pass.pop_debug_group();
  }
  fn render(&mut self, pass: &mut FrameRenderPass);
}
impl PassContent for Box<dyn PassContent + '_> {
  fn debug_label(&self) -> String {
    (**self).debug_label()
  }
  fn render(&mut self, pass: &mut FrameRenderPass) {
    (**self).render(pass);
  }
}

pub struct ActiveRenderPass {
  pub pass: FrameRenderPass,
  /// hold the attachments, so the pooled attachments will not be reused by others during the
  /// pass recording.
  pub desc: RenderPassDescription,
  pipeline_query_active: bool,
}

impl Drop for ActiveRenderPass {
  fn drop(&mut self) {
    // the pipeline statistics query must be ended in the same pass
    if self.pipeline_query_active {
      self.pass.end_pipeline_statistics_query();
    }
  }
}

impl ActiveRenderPass {
  pub fn by_if(self, renderable: &mut Option<impl PassContent>) -> Self {
    if let Some(renderable) = renderable {
      return self.by(renderable);
    }
    self
  }

  pub fn by(mut self, renderable: &mut impl PassContent) -> Self {
    // when we are debug build, enable label scope writing
    #[cfg(debug_assertions)]
    {
      renderable.render_with_scope_label(&mut self.pass);
    }

    #[cfg(not(debug_assertions))]
    {
      renderable.render(&mut self.pass);
    }

    self
  }
}
