// use std::marker::PhantomData;

// use crate::*;

// pub struct Flex {
//   pub direction: bool,
// }

// impl<T, C: Component<T>> ComponentAbility<T, C> for Flex {}

// // impl<T, C> LayoutAble<Vec<T>> for Flex<T, C>
// // where
// //   C: Passthrough<Vec<T>>,
// // {
// //   fn layout(&mut self, constraint: LayoutConstraint) -> LayoutSize {
// //     todo!()
// //   }

// //   fn set_position(&mut self, position: UIPosition) {
// //     todo!()
// //   }
// // }

// pub struct FlexChild<T> {
//   inner: Box<dyn Component<T>>,
// }
