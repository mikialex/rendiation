// use crate::*;

// // struct ReactiveUpdaterGroup<C> {
// //   updater: Vec<Box<dyn ReactiveUpdateNester<C>>>,
// // }

// // impl<C> Default for ReactiveUpdaterGroup<C> {
// //   fn default() -> Self {
// //     Self {
// //       updater: Default::default(),
// //     }
// //   }
// // }

// // impl<C> ReactiveUpdaterGroup<C> {
// //   pub fn with(self, another: impl ReactiveUpdateNester<C> + 'static) -> Self {
// //     todo!()
// //   }
// // }

// // impl<C> ReactiveUpdateNester<C> for ReactiveUpdaterGroup<C> {
// //   fn poll_update_inner(
// //     self: Pin<&mut Self>,
// //     cx: &mut Context<'_>,
// //     inner: &mut C,
// //   ) -> Poll<Option<()>> {
// //     // for updater in &mut self.updater {
// //     //   //
// //     // }
// //     todo!()
// //   }
// // }

// pub trait ReactiveUpdateNesterStreamExt: Stream + Sized {
//   fn bind<F>(self, updater: F) -> StreamToReactiveUpdateNester<F, Self> {
//     StreamToReactiveUpdateNester {
//       updater,
//       stream: self,
//     }
//   }
// }
// impl<T: Stream + Sized> ReactiveUpdateNesterStreamExt for T {}

// pub struct StreamToReactiveUpdateNester<F, S> {
//   updater: F,
//   stream: S,
// }

// impl<C, F, S, T> ReactiveUpdateNester<C> for StreamToReactiveUpdateNester<F, S>
// where
//   S: Stream<Item = T> + Unpin,
//   F: Fn(&mut C, T),
//   Self: Unpin,
// {
//   fn poll_update_inner(
//     mut self: Pin<&mut Self>,
//     cx: &mut Context<'_>,
//     inner: &mut C,
//   ) -> Poll<Option<()>> {
//     self.stream.poll_next_unpin(cx).map(|v| {
//       v.map(|v| {
//         (self.updater)(inner, v);
//       })
//     })
//   }
// }

// impl<C: Eventable, F, S> EventableNester<C> for StreamToReactiveUpdateNester<F, S> {
//   fn event(&mut self, event: &mut EventCtx, inner: &mut C) {
//     inner.event(event);
//   }
// }
// impl<C: Presentable, F, S> PresentableNester<C> for StreamToReactiveUpdateNester<F, S> {
//   fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C) {
//     inner.render(builder);
//   }
// }
// impl<C: HotAreaProvider, F, S> HotAreaNester<C> for StreamToReactiveUpdateNester<F, S> {
//   fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
//     inner.is_point_in(point)
//   }
// }
// impl<C: LayoutAble, F, S> LayoutAbleNester<C> for StreamToReactiveUpdateNester<F, S> {
//   fn layout(
//     &mut self,
//     constraint: LayoutConstraint,
//     ctx: &mut LayoutCtx,
//     inner: &mut C,
//   ) -> LayoutResult {
//     inner.layout(constraint, ctx)
//   }

//   fn set_position(&mut self, position: UIPosition, inner: &mut C) {
//     inner.set_position(position)
//   }
// }
