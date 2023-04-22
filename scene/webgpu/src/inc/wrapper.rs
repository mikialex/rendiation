use crate::*;

#[derive(Copy, Clone)]
pub enum RenderComponentDelta {
  ShaderHash,
  ContentRef,
  Content,
  Draw,
}

pub type ReactiveRenderComponent<T> = impl Stream<Item = RenderComponentDelta>;

#[pin_project::pin_project]
pub struct RenderComponentCell<T> {
  source: EventSource<RenderComponentDelta>,
  #[pin]
  pub inner: T,
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
