use std::mem::ManuallyDrop;

use rendiation_algebra::*;

use crate::*;

pub struct FrameRenderPass {
  pub ctx: GPURenderPassCtx,
  pub pass_info_pool: PassInfoPool,
  /// access pass_info_pool requires locking, so we keep here to reduce the locking to pass level
  pub pass_info: UniformBufferDataView<RenderPassGPUInfoData>,
}

impl FrameRenderPass {
  pub fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32, min_depth: f32, max_depth: f32) {
    self.ctx.pass.set_viewport(x, y, w, h, min_depth, max_depth);
    self.pass_info = self
      .pass_info_pool
      .get_pass_info((w, h).into(), &self.ctx.gpu.device);
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

/// Create a pass descriptor with given name. The provide name is used for debug purpose, not
/// required to be unique
pub fn pass(name: impl Into<String>) -> RenderPassDescription {
  RenderPassDescription {
    name: name.into(),
    ..Default::default()
  }
}

#[derive(Default, Clone)]
pub struct RenderPassDescription {
  pub name: String,
  pub channels: Vec<(gpu::Operations<gpu::Color>, RenderTargetView)>,
  /// (depth_op, stencil_op, attachment)
  pub depth_stencil_target: Option<(gpu::Operations<f32>, gpu::Operations<u32>, RenderTargetView)>,
  pub resolve_target: Option<RenderTargetView>,
}

impl RenderPassDescription {
  pub fn make_all_channel_and_depth_into_load_op(&mut self) {
    for c in self.channels.iter_mut() {
      c.0.load = gpu::LoadOp::Load;
    }
    if let Some(c) = self.depth_stencil_target.as_mut() {
      c.0.load = gpu::LoadOp::Load;
    }
  }

  pub fn buffer_size(&self) -> Vec2<f32> {
    self
      .channels
      .first()
      .map(|c| &c.1)
      .or_else(|| self.depth_stencil_target.as_ref().map(|c| &c.2))
      .map(|c| Vec2::from(c.size().into_usize()).map(|v| v as f32))
      .unwrap()
  }

  #[must_use]
  pub fn with_name(mut self, name: &str) -> Self {
    self.name = name.to_string();
    self
  }

  #[must_use]
  pub fn with_color(
    mut self,
    attachment: &RenderTargetView,
    op: impl Into<gpu::Operations<gpu::Color>>,
  ) -> Self {
    self.push_color(attachment, op);
    self
  }

  pub fn push_color(
    &mut self,
    attachment: &RenderTargetView,
    op: impl Into<gpu::Operations<gpu::Color>>,
  ) -> usize {
    let idx = self.channels.len();
    self.channels.push((op.into(), attachment.clone()));
    idx
  }

  /// if the attachment has no stencil, stencil_op will be ignored, same as the depth_op
  #[must_use]
  pub fn with_depth(
    mut self,
    attachment: &RenderTargetView,
    depth_op: impl Into<gpu::Operations<f32>>,
    stencil_op: impl Into<gpu::Operations<u32>>,
  ) -> Self {
    self.set_depth(attachment, depth_op, stencil_op);
    self
  }

  /// if the attachment has no stencil, stencil_op will be ignored, same as the depth_op
  pub fn set_depth(
    &mut self,
    attachment: &RenderTargetView,
    depth_op: impl Into<gpu::Operations<f32>>,
    stencil_op: impl Into<gpu::Operations<u32>>,
  ) {
    self
      .depth_stencil_target
      .replace((depth_op.into(), stencil_op.into(), attachment.clone()));
  }

  #[must_use]
  pub fn resolve_to(mut self, attachment: &RenderTargetView) -> Self {
    self.resolve_target = Some(attachment.clone());
    self
  }

  #[must_use]
  pub fn render_ctx(self, ctx: &mut FrameCtx) -> ActiveRenderPass {
    self.render(
      &mut ctx.encoder,
      ctx.gpu,
      ctx.statistics.as_ref(),
      Some(ctx.pass_info_pool.clone()),
    )
  }

  #[must_use]
  pub fn render(
    self,
    encoder: &mut GPUCommandEncoder,
    gpu: &GPU,
    measurement_resolver: Option<&FrameStaticInfoResolver>,
    pass_info_pool: Option<PassInfoPool>,
  ) -> ActiveRenderPass {
    let mut pass = encoder.begin_render_pass_with_info(
      self.clone(),
      gpu.clone(),
      measurement_resolver
        .and_then(|r| r.time_query_supported.then_some(()))
        .is_some(),
      pass_info_pool.unwrap_or_default(),
    );

    let measurement = measurement_resolver.map(|m| m.create_defer_logic(&mut pass, gpu));

    ActiveRenderPass {
      desc: self,
      pass: ManuallyDrop::new(pass),
      measurement,
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
  pub pass: ManuallyDrop<FrameRenderPass>,
  pub desc: RenderPassDescription,
  pub measurement: Option<PassMeasurementDeferLogic>,
}

impl Drop for ActiveRenderPass {
  fn drop(&mut self) {
    if let Some(measurement) = self.measurement.as_mut() {
      measurement.resolve_pipeline_stat(&mut self.pass, &self.desc);
    }
    let time = self.pass.time_measuring.take();
    unsafe { ManuallyDrop::drop(&mut self.pass) };
    if let Some(measurement) = self.measurement.as_mut() {
      measurement.resolve_pass_timing(time, &self.desc);
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

pub fn color(r: f64, g: f64, b: f64, a: f64) -> gpu::Color {
  gpu::Color { r, g, b, a }
}
pub fn color_same(r: f64) -> gpu::Color {
  color(r, r, r, r)
}

pub fn all_zero() -> gpu::Color {
  color_same(0.)
}

pub fn clear_and_store<V>(v: V) -> gpu::Operations<V> {
  gpu::Operations {
    load: gpu::LoadOp::Clear(v),
    store: gpu::StoreOp::Store,
  }
}

/// implementation is same as [clear_and_store] but use the default all zero clear value.
///
/// user should use this if the writes guarantee cover the full frame
pub fn store_full_frame<V: Default>() -> gpu::Operations<V> {
  gpu::Operations {
    load: gpu::LoadOp::Clear(V::default()),
    store: gpu::StoreOp::Store,
  }
}

pub fn load_and_store<V>() -> gpu::Operations<V> {
  gpu::Operations {
    load: gpu::LoadOp::Load,
    store: gpu::StoreOp::Store,
  }
}

/// if attachment result is not read by subsequent passes use this can optimize performance in TBDR arch
/// The write result is persist between the drawcall in this pass, but not available to subsequent passes
///
/// It's relatively rare to use
pub fn load_once_and_discard<V>() -> gpu::Operations<V> {
  gpu::Operations {
    load: gpu::LoadOp::Load,
    store: gpu::StoreOp::Discard,
  }
}
