use std::marker::PhantomData;

use crate::ui::{Component, LayoutAble, LayoutConstraint, LayoutSize, UIPosition};

struct Flex<T, C> {
  inner: C,
  phantom: PhantomData<T>,
}

// impl<C, T> Component<Vec<T>> for Flex<T, C>
// where
//   C: Passthrough<T>,
// {
//   fn update(&mut self, model: &Vec<T>) {}
// }

// impl<T, C> Passthrough<Vec<T>> for Flex<T, C>
// where
//   C: Passthrough<Vec<T>>,
// {
//   fn visit(&self, f: impl FnMut(&dyn Component<Vec<T>>)) {
//     self.inner.visit(f)
//   }

//   fn mutate(&mut self, f: impl FnMut(&mut dyn Component<Vec<T>>)) {
//     self.inner.mutate(f)
//   }
// }

// impl<T, C> LayoutAble<Vec<T>> for Flex<T, C>
// where
//   C: Passthrough<Vec<T>>,
// {
//   fn layout(&mut self, constraint: LayoutConstraint) -> LayoutSize {
//     todo!()
//   }

//   fn set_position(&mut self, position: UIPosition) {
//     todo!()
//   }
// }

struct FlexChild<T> {
  inner: Box<dyn Component<T>>,
}
