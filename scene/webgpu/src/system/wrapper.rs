#![allow(non_upper_case_globals)]

use __core::ops::DerefMut;

use crate::*;

pub trait ReactiveRenderComponent: RenderComponentAny {
  // we could remove this box in future
  fn create_render_component_delta_stream(
    &self,
  ) -> Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>;
}

pub trait ReactiveRenderComponentSource: Stream<Item = RenderComponentDeltaFlag> + Unpin {
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent;
}

bitflags::bitflags! {
  #[derive(Default)]
  pub struct RenderComponentDeltaFlag: u32 {
    const ShaderHash = 0b00000001;
    const ContentRef = 0b00000010;
    const Content =    0b00000100;
    const Draw =       0b00001000;

    const RefAndHash = Self::ContentRef.bits() | Self::ShaderHash.bits();
  }
}

pub type RenderComponentDeltaStream<T> = impl Stream<Item = RenderComponentDeltaFlag>;

#[pin_project::pin_project]
pub struct RenderComponentCell<T> {
  source: EventSource<RenderComponentDeltaFlag>,
  #[pin]
  pub inner: T,
}

impl<T> ReactiveRenderComponent for RenderComponentCell<T>
where
  T: RenderComponent + Stream<Item = RenderComponentDeltaFlag> + Unpin + 'static,
{
  fn create_render_component_delta_stream(
    &self,
  ) -> Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>> {
    Box::pin(self.create_render_component_delta_stream())
  }
}

impl<T: ShaderHashProvider> ShaderHashProvider for RenderComponentCell<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.inner.hash_pipeline(hasher)
  }
}
impl<T: ShaderPassBuilder> ShaderPassBuilder for RenderComponentCell<T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.inner.setup_pass(ctx)
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.inner.post_setup_pass(ctx)
  }
}
impl<T: ShaderGraphProvider> ShaderGraphProvider for RenderComponentCell<T> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.inner.build(builder)
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.inner.post_build(builder)
  }
}

impl<T> Deref for RenderComponentCell<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T> DerefMut for RenderComponentCell<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl<T: Stream> Stream for RenderComponentCell<T> {
  type Item = T::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
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

  pub fn create_render_component_delta_stream(&self) -> RenderComponentDeltaStream<T> {
    self
      .source
      .listen_by(|v| *v, RenderComponentDeltaFlag::all())
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
