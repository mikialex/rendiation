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

/// The alignment of the widgets on the container's cross (or minor) axis.
///
/// If a widget is smaller than the container on the minor axis, this determines
/// where it is positioned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CrossAxisAlignment {
  /// Top or leading.
  ///
  /// In a vertical container, widgets are top aligned. In a horiziontal
  /// container, their leading edges are aligned.
  Start,
  /// Widgets are centered in the container.
  Center,
  /// Bottom or trailing.
  ///
  /// In a vertical container, widgets are bottom aligned. In a horiziontal
  /// container, their trailing edges are aligned.
  End,
  /// Align on the baseline.
  ///
  /// In a horizontal container, widgets are aligned along the calculated
  /// baseline. In a vertical container, this is equivalent to `End`.
  ///
  /// The calculated baseline is the maximum baseline offset of the children.
  Baseline,
  /// Fill the available space.
  ///
  /// The size on this axis is the size of the largest widget;
  /// other widgets must fill that space.
  Fill,
}

/// Arrangement of children on the main axis.
///
/// If there is surplus space on the main axis after laying out children, this
/// enum represents how children are laid out in this space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainAxisAlignment {
  /// Top or leading.
  ///
  /// Children are aligned with the top or leading edge, without padding.
  Start,
  /// Children are centered, without padding.
  Center,
  /// Bottom or trailing.
  ///
  /// Children are aligned with the bottom or trailing edge, without padding.
  End,
  /// Extra space is divided evenly between each child.
  SpaceBetween,
  /// Extra space is divided evenly between each child, as well as at the ends.
  SpaceEvenly,
  /// Space between each child, with less at the start and end.
  ///
  /// This divides space such that each child is separated by `n` units,
  /// and the start and end have `n/2` units of padding.
  SpaceAround,
}
