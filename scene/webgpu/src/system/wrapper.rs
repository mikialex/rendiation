#![allow(non_upper_case_globals)]

use core::ops::DerefMut;

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
  #[derive(Default, Copy, Clone, PartialEq, Eq)]
  pub struct RenderComponentDeltaFlag: u32 {
    const ShaderHash = 0b00000001;
    const ContentRef = 0b00000010;
    const Content =    0b00000100;
    const Draw =       0b00001000;

    const RefAndHash = Self::ContentRef.bits() | Self::ShaderHash.bits();
  }
}

pub type RenderComponentDeltaStream<T> = impl Stream<Item = RenderComponentDeltaFlag>;

impl RenderComponentDeltaFlag {
  pub fn into_poll(self) -> Poll<Option<Self>> {
    if self != Default::default() {
      Poll::Ready(Some(self))
    } else {
      Poll::Pending
    }
  }
}

pub trait PollOr {
  fn poll_or(self, other: Self) -> Self;
}
impl PollOr for RenderComponentDeltaFlag {
  fn poll_or(self, other: Self) -> Self {
    self | other
  }
}
impl PollOr for () {
  fn poll_or(self, _: Self) -> Self {}
}

pub trait PollResultUtil<T> {
  fn p_or(self, other: Self) -> Poll<Option<T>>;
}
impl<T: PollOr> PollResultUtil<T> for Poll<Option<T>> {
  fn p_or(self, other: Self) -> Self {
    match (self, other) {
      (Poll::Ready(a), Poll::Ready(b)) => Poll::Ready(a.zip(b).map(|(a, b)| a.poll_or(b))),
      (Poll::Ready(a), Poll::Pending) => Poll::Ready(a),
      (Poll::Pending, Poll::Ready(b)) => Poll::Ready(b),
      (Poll::Pending, Poll::Pending) => Poll::Pending,
    }
  }
}
impl<T: PollOr> PollResultUtil<T> for Option<Poll<Option<T>>> {
  fn p_or(self, other: Self) -> Poll<Option<T>> {
    match (self, other) {
      (None, None) => Poll::Pending,
      (None, Some(b)) => b,
      (Some(a), None) => a,
      (Some(a), Some(b)) => a.p_or(b),
    }
  }
}

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
impl<T: GraphicsShaderProvider> GraphicsShaderProvider for RenderComponentCell<T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.inner.build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
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
      .unbound_listen_by(|v| *v, |v| v(RenderComponentDeltaFlag::all()))
  }
}

#[macro_export]
macro_rules! poll_update_texture_handle_uniform {
  ($this: tt, $name: tt, $cx: tt, $flag: tt, $uniform_flag: tt) => {
    $this.$name.poll_change($cx, &mut $flag, |d| {
      $this.uniform.mutate(|v| {
        v.$name.apply(d).ok();
        $uniform_flag = true;
      });
    });
  };
}
