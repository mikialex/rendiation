// use std::marker::PhantomData;

use crate::*;

pub struct Flex<T> {
  direction: Axis,
  cross_alignment: CrossAxisAlignment,
  main_alignment: MainAxisAlignment,
  fill_major_axis: bool,
  children: Vec<Child<T>>,
}

enum Child<T> {
  Fixed {
    widget: Box<dyn Component<T>>,
    alignment: Option<CrossAxisAlignment>,
  },
  Flex {
    widget: Box<dyn Component<T>>,
    alignment: Option<CrossAxisAlignment>,
    flex: f64,
  },
  FixedSpacer(f64, f64),
  FlexedSpacer(f64, f64),
}

/// An axis in visual space.
///
/// Most often used by widgets to describe
/// the direction in which they grow as their number of children increases.
/// Has some methods for manipulating geometry with respect to the axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis {
  /// The x axis
  Horizontal,
  /// The y axis
  Vertical,
}

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

// impl<T> LayoutAble for Flex<T> {
//   fn layout(&mut self, bc: LayoutConstraint, ctx: &mut LayoutCtx) -> LayoutSize {
//     // we loosen our constraints when passing to children.
//     let loosened_bc = bc.loosen();

//     // minor-axis values for all children
//     let mut minor = self.direction.minor(bc.min());
//     // these two are calculated but only used if we're baseline aligned
//     let mut max_above_baseline = 0f64;
//     let mut max_below_baseline = 0f64;
//     let mut any_use_baseline = self.cross_alignment == CrossAxisAlignment::Baseline;

//     // Measure non-flex children.
//     let mut major_non_flex = 0.0;
//     let mut flex_sum = 0.0;
//     for child in &mut self.children {
//       match child {
//         Child::Fixed { widget, alignment } => {
//           any_use_baseline &= *alignment == Some(CrossAxisAlignment::Baseline);

//           let child_bc = self
//             .direction
//             .constraints(&loosened_bc, 0.0, std::f64::INFINITY);
//           let child_size = widget.layout(ctx, &child_bc, data, env);
//           let baseline_offset = widget.baseline_offset();

//           major_non_flex += self.direction.major(child_size).expand();
//           minor = minor.max(self.direction.minor(child_size).expand());
//           max_above_baseline = max_above_baseline.max(child_size.height - baseline_offset);
//           max_below_baseline = max_below_baseline.max(baseline_offset);
//         }
//         Child::FixedSpacer(kv, calculated_siz) => {
//           *calculated_siz = kv.resolve(env);
//           *calculated_siz = calculated_siz.max(0.0);
//           major_non_flex += *calculated_siz;
//         }
//         Child::Flex { flex, .. } | Child::FlexedSpacer(flex, _) => flex_sum += *flex,
//       }
//     }

//     let total_major = self.direction.major(bc.max());
//     let remaining = (total_major - major_non_flex).max(0.0);
//     let mut remainder: f64 = 0.0;

//     let mut major_flex: f64 = 0.0;
//     let px_per_flex = remaining / flex_sum;
//     // Measure flex children.
//     for child in &mut self.children {
//       match child {
//         Child::Flex { widget, flex, .. } => {
//           let desired_major = (*flex) * px_per_flex + remainder;
//           let actual_major = desired_major.round();
//           remainder = desired_major - actual_major;

//           let child_bc = self.direction.constraints(&loosened_bc, 0.0, actual_major);
//           let child_size = widget.layout(ctx, &child_bc, data, env);
//           let baseline_offset = widget.baseline_offset();

//           major_flex += self.direction.major(child_size).expand();
//           minor = minor.max(self.direction.minor(child_size).expand());
//           max_above_baseline = max_above_baseline.max(child_size.height - baseline_offset);
//           max_below_baseline = max_below_baseline.max(baseline_offset);
//         }
//         Child::FlexedSpacer(flex, calculated_size) => {
//           let desired_major = (*flex) * px_per_flex + remainder;
//           *calculated_size = desired_major.round();
//           remainder = desired_major - *calculated_size;
//           major_flex += *calculated_size;
//         }
//         _ => {}
//       }
//     }

//     // figure out if we have extra space on major axis, and if so how to use it
//     let extra = if self.fill_major_axis {
//       (remaining - major_flex).max(0.0)
//     } else {
//       // if we are *not* expected to fill our available space this usually
//       // means we don't have any extra, unless dictated by our constraints.
//       (self.direction.major(bc.min()) - (major_non_flex + major_flex)).max(0.0)
//     };

//     let mut spacing = Spacing::new(self.main_alignment, extra, self.children.len());

//     // the actual size needed to tightly fit the children on the minor axis.
//     // Unlike the 'minor' var, this ignores the incoming constraints.
//     let minor_dim = match self.direction {
//       Axis::Horizontal if any_use_baseline => max_below_baseline + max_above_baseline,
//       _ => minor,
//     };

//     let extra_height = minor - minor_dim.min(minor);

//     let mut major = spacing.next().unwrap_or(0.);
//     let mut child_paint_rect = Rect::ZERO;

//     for child in &mut self.children {
//       match child {
//         Child::Fixed { widget, alignment }
//         | Child::Flex {
//           widget, alignment, ..
//         } => {
//           let child_size = widget.layout_rect().size();
//           let alignment = alignment.unwrap_or(self.cross_alignment);
//           let child_minor_offset = match alignment {
//             // This will ignore baseline alignment if it is overridden on children,
//             // but is not the default for the container. Is this okay?
//             CrossAxisAlignment::Baseline if matches!(self.direction, Axis::Horizontal) => {
//               let child_baseline = widget.baseline_offset();
//               let child_above_baseline = child_size.height - child_baseline;
//               extra_height + (max_above_baseline - child_above_baseline)
//             }
//             CrossAxisAlignment::Fill => {
//               let fill_size: Size = self
//                 .direction
//                 .pack(self.direction.major(child_size), minor_dim)
//                 .into();
//               let child_bc = BoxConstraints::tight(fill_size);
//               widget.layout(ctx, &child_bc, data, env);
//               0.0
//             }
//             _ => {
//               let extra_minor = minor_dim - self.direction.minor(child_size);
//               alignment.align(extra_minor)
//             }
//           };

//           let child_pos: Point = self.direction.pack(major, child_minor_offset).into();
//           widget.set_origin(ctx, data, env, child_pos);
//           child_paint_rect = child_paint_rect.union(widget.paint_rect());
//           major += self.direction.major(child_size).expand();
//           major += spacing.next().unwrap_or(0.);
//         }
//         Child::FlexedSpacer(_, calculated_size) | Child::FixedSpacer(_, calculated_size) => {
//           major += *calculated_size;
//         }
//       }
//     }

//     if flex_sum > 0.0 && total_major.is_infinite() {
//       tracing::warn!("A child of Flex is flex, but Flex is unbounded.")
//     }

//     if flex_sum > 0.0 {
//       major = total_major;
//     }

//     let my_size: Size = self.direction.pack(major, minor_dim).into();

//     // if we don't have to fill the main axis, we loosen that axis before constraining
//     let my_size = if !self.fill_major_axis {
//       let max_major = self.direction.major(bc.max());
//       self
//         .direction
//         .constraints(bc, 0.0, max_major)
//         .constrain(my_size)
//     } else {
//       bc.constrain(my_size)
//     };

//     let my_bounds = Rect::ZERO.with_size(my_size);
//     let insets = child_paint_rect - my_bounds;
//     ctx.set_paint_insets(insets);

//     let baseline_offset = match self.direction {
//       Axis::Horizontal => max_below_baseline,
//       Axis::Vertical => (&self.children)
//         .last()
//         .map(|last| {
//           let child = last.widget();
//           if let Some(widget) = child {
//             let child_bl = widget.baseline_offset();
//             let child_max_y = widget.layout_rect().max_y();
//             let extra_bottom_padding = my_size.height - child_max_y;
//             child_bl + extra_bottom_padding
//           } else {
//             0.0
//           }
//         })
//         .unwrap_or(0.0),
//     };

//     ctx.set_baseline_offset(baseline_offset);
//     my_size
//   }
// }
