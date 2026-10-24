use rendiation_algebra::*;

use crate::*;

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
  /// (op, attachment, resolve_target)
  pub channels: Vec<(
    gpu::Operations<gpu::Color>,
    RenderTargetView,
    Option<RenderTargetView>,
  )>,
  /// (depth_op, stencil_op, attachment), None op means the aspect is read only, see
  /// [RenderPassDescription::with_depth]
  pub depth_stencil_target: Option<(
    Option<gpu::Operations<f32>>,
    Option<gpu::Operations<u32>>,
    RenderTargetView,
  )>,
}

impl RenderPassDescription {
  /// the first color attachment or the depth attachment, all attachments should have the same
  /// size and sample count
  fn first_attachment(&self) -> Option<&RenderTargetView> {
    self
      .channels
      .first()
      .map(|c| &c.1)
      .or_else(|| self.depth_stencil_target.as_ref().map(|c| &c.2))
  }

  pub fn sample_count(&self) -> Option<u32> {
    self.first_attachment().map(|v| v.sample_count())
  }

  /// # Panics
  ///
  /// Panics if the description has no attachment.
  pub fn buffer_size(&self) -> Vec2<f32> {
    let size = self
      .first_attachment()
      .expect("render pass should have at least one attachment")
      .size();
    Vec2::from(size.into_usize()).map(|v| v as f32)
  }

  pub fn make_all_channel_and_depth_into_load_op(&mut self) {
    for c in self.channels.iter_mut() {
      c.0.load = gpu::LoadOp::Load;
    }
    if let Some(op) = self
      .depth_stencil_target
      .as_mut()
      .and_then(|c| c.0.as_mut())
    {
      op.load = gpu::LoadOp::Load;
    }
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

  #[must_use]
  pub fn with_color_and_resolve_target(
    mut self,
    attachment: &RenderTargetView,
    op: impl Into<gpu::Operations<gpu::Color>>,
    resolve_target: &RenderTargetView,
  ) -> Self {
    self.push_color_with_resolve_target_some(attachment, op, Some(resolve_target.clone()));
    self
  }

  pub fn push_color(
    &mut self,
    attachment: &RenderTargetView,
    op: impl Into<gpu::Operations<gpu::Color>>,
  ) -> usize {
    let idx = self.channels.len();
    self.channels.push((op.into(), attachment.clone(), None));
    idx
  }

  pub fn push_color_with_resolve_target_some(
    &mut self,
    attachment: &RenderTargetView,
    op: impl Into<gpu::Operations<gpu::Color>>,
    resolve_target: Option<RenderTargetView>,
  ) -> usize {
    let idx = self.channels.len();
    self
      .channels
      .push((op.into(), attachment.clone(), resolve_target));
    idx
  }

  /// if the attachment has no stencil, stencil_op will be ignored, same as the depth_op
  ///
  /// passing None op makes the aspect read only in this pass, the content is loaded and kept. The
  /// read only attachment can be bound as texture and accessed in the same pass at the same time,
  /// and all pipelines used in this pass must not write it.
  #[must_use]
  pub fn with_depth(
    mut self,
    attachment: &RenderTargetView,
    depth_op: impl Into<Option<gpu::Operations<f32>>>,
    stencil_op: impl Into<Option<gpu::Operations<u32>>>,
  ) -> Self {
    self.set_depth(attachment, depth_op, stencil_op);
    self
  }

  /// see [RenderPassDescription::with_depth]
  pub fn set_depth(
    &mut self,
    attachment: &RenderTargetView,
    depth_op: impl Into<Option<gpu::Operations<f32>>>,
    stencil_op: impl Into<Option<gpu::Operations<u32>>>,
  ) {
    self
      .depth_stencil_target
      .replace((depth_op.into(), stencil_op.into(), attachment.clone()));
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
pub fn load_once_and_discard<V>() -> gpu::Operations<V> {
  gpu::Operations {
    load: gpu::LoadOp::Load,
    store: gpu::StoreOp::Discard,
  }
}
