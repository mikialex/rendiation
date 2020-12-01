use crate::{component::Component, ComponentTree};
use core::marker::PhantomData;
use super::Lens;

pub struct LensWrap<U, L, W> {
  inner: W,
  lens: L,
  // The following is a workaround for otherwise getting E0207.
  phantom: PhantomData<U>,
}

impl<U, L, W> LensWrap<U, L, W> {
  /// Wrap a widget with a lens.
  ///
  /// When the lens has type `Lens<T, U>`, the inner widget has data
  /// of type `U`, and the wrapped widget has data of type `T`.
  pub fn new(inner: W, lens: L) -> LensWrap<U, L, W> {
    LensWrap {
      inner,
      lens,
      phantom: Default::default(),
    }
  }
}

impl<T, U, L, W> Component<T> for LensWrap<U, L, W>
where
  L: Lens<T, U>,
  W: Component<U>,
{
  fn render(&self) -> ComponentTree<T> {
      todo!()
  }
}
