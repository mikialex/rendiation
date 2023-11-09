use crate::*;

trivial_stream_impl!(Flex);
impl<C: View> ViewNester<C> for Flex
where
  for<'a> &'a mut C: IntoIterator<Item = &'a mut Child>,
  for<'a> <&'a mut C as IntoIterator>::IntoIter: ExactSizeIterator,
{
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut C) {
    match detail {
      ViewRequest::Layout(p) => match p {
        LayoutProtocol::DoLayout {
          constraint,
          output,
          ctx,
        } => **output = self.layout_impl(*constraint, ctx, inner),
        LayoutProtocol::PositionAt(position) => self.set_position_impl(*position),
      },
      ViewRequest::Encode(builder) => {
        builder.push_translate(self.layout.relative_position);
        inner.draw(builder);
        builder.pop_translate()
      }
      _ => inner.request(detail),
    }
  }
}

impl Flex {
  fn layout_impl<C>(
    &mut self,
    bc: LayoutConstraint,
    ctx: &mut LayoutCtx,
    inner: &mut C,
  ) -> LayoutResult
  where
    for<'a> &'a mut C: IntoIterator<Item = &'a mut Child>,
    for<'a> <&'a mut C as IntoIterator>::IntoIter: ExactSizeIterator,
  {
    // we loosen our constraints when passing to children.
    let loosened_bc = bc.loosen();

    // minor-axis values for all children
    let mut minor = self.direction.minor(bc.min());
    // these two are calculated but only used if we're baseline aligned
    let mut max_above_baseline = 0f32;
    let mut max_below_baseline = 0f32;
    let mut any_use_baseline = self.cross_alignment == CrossAxisAlignment::Baseline;

    // Measure non-flex children.
    let mut major_non_flex = 0.0;
    let mut flex_sum = 0.0;
    for child in inner.into_iter() {
      match child {
        Child::Fixed {
          widget,
          alignment,
          result,
          ..
        } => {
          any_use_baseline &= *alignment == Some(CrossAxisAlignment::Baseline);

          let child_bc = self
            .direction
            .constraints(&loosened_bc, 0.0, std::f32::INFINITY);
          let child_layout = widget.layout(child_bc, ctx);
          let child_size = child_layout.size;
          let baseline_offset = child_layout.baseline_offset;
          *result = child_layout;

          major_non_flex += self.direction.major(child_size);
          minor = minor.max(self.direction.minor(child_size));
          max_above_baseline = max_above_baseline.max(child_size.height - baseline_offset);
          max_below_baseline = max_below_baseline.max(baseline_offset);
        }
        Child::FixedSpacer(kv, calculated_siz) => {
          *calculated_siz = *kv;
          *calculated_siz = calculated_siz.max(0.0);
          major_non_flex += *calculated_siz;
        }
        Child::Flex { flex, .. } | Child::FlexedSpacer(flex, _) => flex_sum += *flex,
      }
    }

    let total_major = self.direction.major(bc.max());
    let remaining = (total_major - major_non_flex).max(0.0);
    let mut remainder: f32 = 0.0;

    let mut major_flex: f32 = 0.0;
    let px_per_flex = remaining / flex_sum;
    // Measure flex children.
    for child in &mut inner.into_iter() {
      match child {
        Child::Flex {
          widget,
          flex,
          result,
          ..
        } => {
          let desired_major = (*flex) * px_per_flex + remainder;
          let actual_major = desired_major.round();
          remainder = desired_major - actual_major;

          let child_bc = self.direction.constraints(&loosened_bc, 0.0, actual_major);

          let child_layout = widget.layout(child_bc, ctx);
          let child_size = child_layout.size;
          let baseline_offset = child_layout.baseline_offset;
          *result = child_layout;

          major_flex += self.direction.major(child_size);
          minor = minor.max(self.direction.minor(child_size));
          max_above_baseline = max_above_baseline.max(child_size.height - baseline_offset);
          max_below_baseline = max_below_baseline.max(baseline_offset);
        }
        Child::FlexedSpacer(flex, calculated_size) => {
          let desired_major = (*flex) * px_per_flex + remainder;
          *calculated_size = desired_major.round();
          remainder = desired_major - *calculated_size;
          major_flex += *calculated_size;
        }
        _ => {}
      }
    }

    // figure out if we have extra space on major axis, and if so how to use it
    let extra = if self.fill_major_axis {
      (remaining - major_flex).max(0.0)
    } else {
      // if we are *not* expected to fill our available space this usually
      // means we don't have any extra, unless dictated by our constraints.
      (self.direction.major(bc.min()) - (major_non_flex + major_flex)).max(0.0)
    };

    let mut spacing = Spacing::new(self.main_alignment, extra, inner.into_iter().len());

    // the actual size needed to tightly fit the children on the minor axis.
    // Unlike the 'minor' var, this ignores the incoming constraints.
    let minor_dim = match self.direction {
      Axis::Horizontal if any_use_baseline => max_below_baseline + max_above_baseline,
      _ => minor,
    };

    let extra_height = minor - minor_dim.min(minor);

    let mut major = spacing.next().unwrap_or(0.);

    for child in inner.into_iter() {
      match child {
        Child::Fixed {
          widget,
          alignment,
          result,
          position,
          ..
        }
        | Child::Flex {
          widget,
          alignment,
          result,
          position,
          ..
        } => {
          let child_size = result.size;
          let alignment = alignment.unwrap_or(self.cross_alignment);
          let child_minor_offset = match alignment {
            // This will ignore baseline alignment if it is overridden on children,
            // but is not the default for the container. Is this okay?
            CrossAxisAlignment::Baseline if matches!(self.direction, Axis::Horizontal) => {
              let child_baseline = result.baseline_offset;
              let child_above_baseline = child_size.height - child_baseline;
              extra_height + (max_above_baseline - child_above_baseline)
            }
            CrossAxisAlignment::Fill => {
              let fill_size: UISize = self
                .direction
                .pack(self.direction.major(child_size), minor_dim)
                .into();
              let child_bc = LayoutConstraint::tight(fill_size);
              widget.layout(child_bc, ctx);
              0.0
            }
            _ => {
              let extra_minor = minor_dim - self.direction.minor(child_size);
              alignment.align(extra_minor)
            }
          };

          let child_pos = self.direction.pack(major, child_minor_offset).into();
          widget.set_position(child_pos);
          *position = child_pos;
          major += self.direction.major(child_size);
          major += spacing.next().unwrap_or(0.);
        }
        Child::FlexedSpacer(_, calculated_size) | Child::FixedSpacer(_, calculated_size) => {
          major += *calculated_size;
        }
      }
    }

    if flex_sum > 0.0 {
      major = total_major;
    }

    let my_size: UISize = self.direction.pack(major, minor_dim).into();

    // if we don't have to fill the main axis, we loosen that axis before constraining
    let my_size = if !self.fill_major_axis {
      let max_major = self.direction.major(bc.max());
      self
        .direction
        .constraints(&bc, 0.0, max_major)
        .constrain(my_size)
    } else {
      bc.constrain(my_size)
    };

    let baseline_offset = match self.direction {
      Axis::Horizontal => max_below_baseline,
      Axis::Vertical => inner
        .into_iter()
        .last() // todo optimize?
        .map(|last| {
          let child = last.widget();
          if let Some((_, result, position)) = child {
            let child_bl = result.baseline_offset;
            let child_max_y = position.y + result.size.height;
            let extra_bottom_padding = my_size.height - child_max_y;
            child_bl + extra_bottom_padding
          } else {
            0.0
          }
        })
        .unwrap_or(0.0),
    };

    self.layout.baseline_offset = baseline_offset;
    self.layout.size = my_size;
    LayoutResult {
      size: my_size,
      baseline_offset,
    }
  }

  fn set_position_impl(&mut self, position: UIPosition) {
    self.layout.set_relative_position(position);
  }
}

struct Spacing {
  alignment: MainAxisAlignment,
  extra: f32,
  n_children: usize,
  index: usize,
  equal_space: f32,
  remainder: f32,
}

impl Spacing {
  /// Given the provided extra space and children count,
  /// this returns an iterator of `f32` spacing,
  /// where the first element is the spacing before any children
  /// and all subsequent elements are the spacing after children.
  fn new(alignment: MainAxisAlignment, extra: f32, n_children: usize) -> Spacing {
    let extra = if extra.is_finite() { extra } else { 0. };
    let equal_space = if n_children > 0 {
      match alignment {
        MainAxisAlignment::Center => extra / 2.,
        MainAxisAlignment::SpaceBetween => extra / (n_children - 1).max(1) as f32,
        MainAxisAlignment::SpaceEvenly => extra / (n_children + 1) as f32,
        MainAxisAlignment::SpaceAround => extra / (2 * n_children) as f32,
        _ => 0.,
      }
    } else {
      0.
    };
    Spacing {
      alignment,
      extra,
      n_children,
      index: 0,
      equal_space,
      remainder: 0.,
    }
  }

  fn next_space(&mut self) -> f32 {
    let desired_space = self.equal_space + self.remainder;
    let actual_space = desired_space.round();
    self.remainder = desired_space - actual_space;
    actual_space
  }
}

impl Iterator for Spacing {
  type Item = f32;

  fn next(&mut self) -> Option<f32> {
    if self.index > self.n_children {
      return None;
    }
    let result = {
      if self.n_children == 0 {
        self.extra
      } else {
        match self.alignment {
          MainAxisAlignment::Start => match self.index == self.n_children {
            true => self.extra,
            false => 0.,
          },
          MainAxisAlignment::End => match self.index == 0 {
            true => self.extra,
            false => 0.,
          },
          MainAxisAlignment::Center => match self.index {
            0 => self.next_space(),
            i if i == self.n_children => self.next_space(),
            _ => 0.,
          },
          MainAxisAlignment::SpaceBetween => match self.index {
            0 => 0.,
            i if i != self.n_children => self.next_space(),
            _ => match self.n_children {
              1 => self.next_space(),
              _ => 0.,
            },
          },
          MainAxisAlignment::SpaceEvenly => self.next_space(),
          MainAxisAlignment::SpaceAround => {
            if self.index == 0 || self.index == self.n_children {
              self.next_space()
            } else {
              self.next_space() + self.next_space()
            }
          }
        }
      }
    };
    self.index += 1;
    Some(result)
  }
}
