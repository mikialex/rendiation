use crate::*;

// struct ReactiveUpdaterGroup<C> {
//   updater: Vec<Box<dyn ReactiveUpdateNester<C>>>,
// }

// impl<C> Default for ReactiveUpdaterGroup<C> {
//   fn default() -> Self {
//     Self {
//       updater: Default::default(),
//     }
//   }
// }

// impl<C> ReactiveUpdaterGroup<C> {
//   pub fn with(self, another: impl ReactiveUpdateNester<C> + 'static) -> Self {
//     todo!()
//   }
// }

// impl<C> ReactiveUpdateNester<C> for ReactiveUpdaterGroup<C> {
//   fn poll_update_inner(
//     self: Pin<&mut Self>,
//     cx: &mut Context<'_>,
//     inner: &mut C,
//   ) -> Poll<Option<()>> {
//     // for updater in &mut self.updater {
//     //   //
//     // }
//     todo!()
//   }
// }

pub trait ReactiveUpdateNesterStreamExt: Stream + Sized {
  fn bind<F>(self, updater: F) -> StreamToReactiveUpdateNester<F, Self> {
    StreamToReactiveUpdateNester {
      updater,
      stream: self,
    }
  }
}
impl<T: Stream + Sized> ReactiveUpdateNesterStreamExt for T {}

pub struct StreamToReactiveUpdateNester<F, S> {
  updater: F,
  stream: S,
}

impl<C, F, S, T> ReactiveUpdateNester<C> for StreamToReactiveUpdateNester<F, S>
where
  S: Stream<Item = T> + Unpin,
  F: Fn(&mut C, T),
  Self: Unpin,
{
  fn poll_update_inner(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    inner: &mut C,
  ) -> Poll<Option<()>> {
    self.stream.poll_next_unpin(cx).map(|v| {
      v.map(|v| {
        (self.updater)(inner, v);
      })
    })
  }
}
