use std::{
  pin::Pin,
  task::{Context, Poll},
};

use crate::*;
use futures::Stream;

#[pin_project::pin_project]
pub struct RenderComponentReactive<T, U> {
  pub gpu: T,
  #[pin]
  pub reactive: U,
  // we could cache shader hash here
}

impl<T, U> RenderComponentReactive<T, U> {
  pub fn new(gpu: T, reactive: U) -> Self {
    Self { gpu, reactive }
  }
  pub fn from_gpu_with_default_reactive(gpu: T) -> Self
  where
    U: Default,
  {
    Self {
      gpu,
      reactive: Default::default(),
    }
  }
}

#[derive(Copy, Clone)]
pub enum RenderComponentDelta {
  ShaderHash,
  ContentRef,
  Content,
  Draw,
}

impl<T, U> Stream for RenderComponentReactive<T, U>
where
  U: Stream<Item = RenderComponentDelta>,
{
  type Item = RenderComponentDelta;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.reactive.poll_next(cx)
  }
}

impl<T: ShaderHashProvider, U> ShaderHashProvider for RenderComponentReactive<T, U> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.gpu.hash_pipeline(hasher)
  }
}

impl<T: ShaderPassBuilder, U> ShaderPassBuilder for RenderComponentReactive<T, U> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.gpu.setup_pass(ctx)
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.gpu.post_setup_pass(ctx)
  }
}

impl<T: ShaderGraphProvider, U> ShaderGraphProvider for RenderComponentReactive<T, U> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.gpu.build(builder)
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.gpu.post_build(builder)
  }
}

#[pin_project::pin_project]
pub struct RenderComponentCell<T> {
  source: EventSource<RenderComponentDelta>,
  #[pin]
  pub inner: T,
}

impl<T: Stream> Stream for RenderComponentCell<T> {
  type Item = T::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.inner.poll_next(cx)
  }
}

impl<T> RenderComponentCell<T> {
  pub fn new(gpu: T) -> Self {
    RenderComponentCell {
      source: Default::default(),
      inner: gpu,
    }
  }

  pub fn create_render_component_delta_stream(&self) -> impl Stream<Item = RenderComponentDelta> {
    self
      .source
      .listen_by(|v| *v, RenderComponentDelta::ContentRef)
  }
}

#[macro_export]
macro_rules! early_return_ready {
  ($e:expr) => {
    match $e {
      $crate::task::Poll::Ready(t) => return $crate::task::Poll::Ready(t),
      $crate::task::Poll::Pending => {}
    }
  };
}

#[macro_export]
macro_rules! early_return_option_ready {
  ($e:expr, $cx:expr ) => {
    if let Some(t) = &mut $e {
      early_return_ready!(t.poll_next_unpin($cx));
    }
  };
}
