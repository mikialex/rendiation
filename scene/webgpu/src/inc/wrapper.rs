#![allow(non_upper_case_globals)]

use __core::ops::DerefMut;

use crate::*;

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

pub type ReactiveRenderComponent<T> = impl Stream<Item = RenderComponentDeltaFlag>;

#[pin_project::pin_project]
pub struct RenderComponentCell<T> {
  source: EventSource<RenderComponentDeltaFlag>,
  #[pin]
  pub inner: T,
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

  pub fn create_render_component_delta_stream(&self) -> ReactiveRenderComponent<T> {
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
