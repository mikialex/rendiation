use std::ops::DerefMut;

use crate::*;

mod layout_impl;

pub fn flex_group() -> ComponentArray<Child> {
  Vec::new().into()
}

pub struct Flex {
  direction: Axis,
  layout: LayoutUnit,
  cross_alignment: CrossAxisAlignment,
  main_alignment: MainAxisAlignment,
  fill_major_axis: bool,
}

impl Flex {
  /// Create a new Flex oriented along the provided axis.
  pub fn for_axis(axis: Axis) -> Self {
    Self {
      layout: Default::default(),
      direction: axis,
      cross_alignment: CrossAxisAlignment::Center,
      main_alignment: MainAxisAlignment::Start,
      fill_major_axis: false,
    }
  }

  /// Create a new horizontal stack.
  ///
  /// The child widgets are laid out horizontally, from left to right.
  pub fn row() -> Self {
    Self::for_axis(Axis::Horizontal)
  }

  /// Create a new vertical stack.
  ///
  /// The child widgets are laid out vertically, from top to bottom.
  pub fn column() -> Self {
    Self::for_axis(Axis::Vertical)
  }

  /// Builder-style method for specifying the childrens' [`CrossAxisAlignment`].
  ///
  /// [`CrossAxisAlignment`]: enum.CrossAxisAlignment.html
  #[must_use]
  pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
    self.cross_alignment = alignment;
    self
  }

  /// Builder-style method for specifying the childrens' [`MainAxisAlignment`].
  ///
  /// [`MainAxisAlignment`]: enum.MainAxisAlignment.html
  #[must_use]
  pub fn main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
    self.main_alignment = alignment;
    self
  }

  /// Builder-style method for setting whether the container must expand
  /// to fill the available space on its main axis.
  ///
  /// If any children have flex then this container will expand to fill all
  /// available space on its main axis; But if no children are flex,
  /// this flag determines whether or not the container should shrink to fit,
  /// or must expand to fill.
  ///
  /// If it expands, and there is extra space left over, that space is
  /// distributed in accordance with the [`MainAxisAlignment`].
  ///
  /// The default value is `false`.
  ///
  /// [`MainAxisAlignment`]: enum.MainAxisAlignment.html
  #[must_use]
  pub fn must_fill_main_axis(mut self, fill: bool) -> Self {
    self.fill_major_axis = fill;
    self
  }
}

pub enum Child {
  Fixed {
    widget: Box<dyn View>,
    result: LayoutResult,
    position: UIPosition,
    alignment: Option<CrossAxisAlignment>,
  },
  Flex {
    widget: Box<dyn View>,
    result: LayoutResult,
    position: UIPosition,
    alignment: Option<CrossAxisAlignment>,
    flex: f32,
  },
  FixedSpacer(f32, f32),
  FlexedSpacer(f32, f32),
}

impl Child {
  pub fn fixed(widget: impl View + 'static) -> Self {
    Self::Fixed {
      widget: Box::new(widget),
      result: Default::default(),
      position: Default::default(),
      alignment: None,
    }
  }

  pub fn flex(widget: impl View + 'static, flex: f32) -> Self {
    Self::Flex {
      widget: Box::new(widget),
      result: Default::default(),
      position: Default::default(),
      alignment: None,
      flex,
    }
  }

  #[must_use]
  pub fn self_alignment(mut self, a: CrossAxisAlignment) -> Self {
    match &mut self {
      Child::Fixed { alignment, .. } => *alignment = a.into(),
      Child::Flex { alignment, .. } => *alignment = a.into(),
      _ => {}
    }
    self
  }
}

impl Stream for Child {
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.deref_mut();
    match this {
      Child::Fixed { widget, .. } => widget.poll_next_unpin(cx),
      Child::Flex { widget, .. } => widget.poll_next_unpin(cx),
      _ => Poll::Pending,
    }
  }
}
impl View for Child {
  fn request(&mut self, detail: &mut ViewRequest) {
    match self {
      Child::Fixed { widget, .. } => widget.request(detail),
      Child::Flex { widget, .. } => widget.request(detail),
      _ => {}
    }
  }
}

impl Child {
  fn widget(&self) -> Option<(&dyn View, &LayoutResult, &UIPosition)> {
    match self {
      Child::Fixed {
        widget,
        result,
        position,
        ..
      } => Some((widget.as_ref(), result, position)),
      Child::Flex {
        widget,
        result,
        position,
        ..
      } => Some((widget.as_ref(), result, position)),
      _ => None,
    }
  }
}

/// An axis in visual space.
///
/// Most often used by widgets to describe
/// the direction in which they grow as their number of children increases.
/// Has some methods for manipulating geometry with respect to the axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
  /// The x axis
  Horizontal,
  /// The y axis
  Vertical,
}

impl Axis {
  /// Get the axis perpendicular to this one.
  #[must_use]
  pub fn cross(self) -> Axis {
    match self {
      Axis::Horizontal => Axis::Vertical,
      Axis::Vertical => Axis::Horizontal,
    }
  }

  /// Extract from the argument the magnitude along this axis
  pub fn major(self, coords: UISize) -> f32 {
    match self {
      Axis::Horizontal => coords.width,
      Axis::Vertical => coords.height,
    }
  }
  /// Extract from the argument the magnitude along the perpendicular axis
  pub fn minor(self, coords: UISize) -> f32 {
    self.cross().major(coords)
  }

  /// Arrange the major and minor measurements with respect to this axis such that it forms
  /// an (x, y) pair.
  pub fn pack(self, major: f32, minor: f32) -> (f32, f32) {
    match self {
      Axis::Horizontal => (major, minor),
      Axis::Vertical => (minor, major),
    }
  }

  /// Generate constraints with new values on the major axis.
  pub(crate) fn constraints(
    self,
    bc: &LayoutConstraint,
    min_major: f32,
    major: f32,
  ) -> LayoutConstraint {
    match self {
      Axis::Horizontal => LayoutConstraint::new(
        UISize::new(min_major, bc.min().height),
        UISize::new(major, bc.max().height),
      ),
      Axis::Vertical => LayoutConstraint::new(
        UISize::new(bc.min().width, min_major),
        UISize::new(bc.max().width, major),
      ),
    }
  }
}

/// The alignment of the widgets on the container's cross (or minor) axis.
///
/// If a widget is smaller than the container on the minor axis, this determines
/// where it is positioned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl CrossAxisAlignment {
  /// Given the difference between the size of the container and the size
  /// of the child (on their minor axis) return the necessary offset for
  /// this alignment.
  fn align(self, val: f32) -> f32 {
    match self {
      CrossAxisAlignment::Start => 0.0,
      // in vertical layout, baseline is equivalent to center
      CrossAxisAlignment::Center | CrossAxisAlignment::Baseline => (val / 2.0).round(),
      CrossAxisAlignment::End => val,
      CrossAxisAlignment::Fill => 0.0,
    }
  }
}

/// Arrangement of children on the main axis.
///
/// If there is surplus space on the main axis after laying out children, this
/// enum represents how children are laid out in this space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
