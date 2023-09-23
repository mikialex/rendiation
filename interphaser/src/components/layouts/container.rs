use crate::*;

/// setup a sized box and use this for positioning child
pub struct Container {
  pub color: DisplayColor,
  pub child_align: ContainerAlignment,
  /// extra relative(parent) offset for self
  pub self_offset: ContainerItemOffset,
  /// extra relative(self) offset for child
  pub child_offset: ContainerItemOffset,
  pub size: ContainerSize,
  /// for simplicity, we only support outer border now
  pub border: RectBorder,
  pub margin: RectBoundaryWidth,
  pub padding: RectBoundaryWidth,
  layout: LayoutUnit,
}

impl Container {
  pub fn sized(size: impl Into<UISize<UILength>>) -> Self {
    Self::size(ContainerSize::ConstraintChild { size: size.into() })
  }

  pub fn padding(mut self, padding: impl Into<RectBoundaryWidth>) -> Self {
    self.padding = padding.into();
    self
  }

  pub fn margin(mut self, margin: impl Into<RectBoundaryWidth>) -> Self {
    self.margin = margin.into();
    self
  }

  pub fn adapt(behavior: AdaptChildSelfBehavior) -> Self {
    Self::size(ContainerSize::AdaptChild { behavior })
  }

  pub fn size(size: ContainerSize) -> Self {
    Self {
      color: (1., 1., 1., 0.).into(),
      child_align: Default::default(),
      self_offset: Default::default(),
      child_offset: Default::default(),
      size,
      layout: Default::default(),
      border: Default::default(),
      margin: Default::default(),
      padding: Default::default(),
    }
  }

  #[must_use]
  pub fn color(mut self, color: DisplayColor) -> Self {
    self.color = color;
    self
  }
  pub fn set_color(&mut self, color: DisplayColor) {
    self.color = color
  }
}

impl<C: View> ViewNester<C> for Container {
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut C) {
    match detail {
      ViewRequest::Layout(p) => match p {
        LayoutProtocol::DoLayout {
          constraint,
          ctx,
          output,
        } => {
          let (self_size, child_size) = self.size.compute_size_pair(*constraint, self, inner, ctx);
          self.layout.size = self_size;
          let align_offset = self.child_align.make_offset(self.layout.size, child_size);

          let position = UIPosition {
            x: align_offset.x
              + self.child_offset.x
              + self.margin.left
              + self.padding.left
              + self.border.width.left,
            y: align_offset.y
              + self.child_offset.y
              + self.margin.top
              + self.padding.top
              + self.border.width.top,
          };
          inner.set_position(position);

          **output = self.layout.size.with_default_baseline();
        }
        LayoutProtocol::PositionAt(p) => {
          let position = (p.x + self.self_offset.x, p.y + self.self_offset.y);
          self.layout.set_relative_position(position.into())
        }
      },
      ViewRequest::Encode(builder) => {
        self.draw(builder);
        builder.push_translate(self.layout.relative_position);
        inner.draw(builder);
        builder.pop_translate()
      }
      ViewRequest::HitTest { point, result } => {
        **result = self.hit_test(*point) || inner.hit_test(*point);
      }
      _ => inner.request(detail),
    }
  }
}

pub enum ContainerSize {
  ConstraintChild { size: UISize<UILength> },
  AdaptChild { behavior: AdaptChildSelfBehavior },
}

pub enum AdaptChildSelfBehavior {
  Max,
  Child,
}

impl ContainerSize {
  /// This used in the container as the view child case
  pub fn compute_size_self(&self, constraint: LayoutConstraint) -> UISize {
    match self {
      ContainerSize::ConstraintChild { size } => {
        let size = size.into_pixel(constraint.max);
        constraint.clamp(size)
      }
      ContainerSize::AdaptChild { behavior } => match behavior {
        AdaptChildSelfBehavior::Max => constraint.max,
        AdaptChildSelfBehavior::Child => constraint.min,
      },
    }
  }

  /// This used in the container as the nester case
  /// (self size, child size)
  pub fn compute_size_pair(
    &self,
    constraint: LayoutConstraint,
    container: &Container,
    child: &mut dyn View,
    ctx: &mut LayoutCtx,
  ) -> (UISize, UISize) {
    match self {
      Self::ConstraintChild { size } => {
        let size = size.into_pixel(constraint.max);

        let size = constraint.clamp(size).inset_boundary(&container.margin);

        let child_max = size
          .inset_boundary(&container.border.width)
          .inset_boundary(&container.padding);

        let child_size = child
          .layout(LayoutConstraint::from_max(child_max), ctx)
          .size;

        (size, child_size)
      }
      Self::AdaptChild { behavior } => {
        let child_constraint = constraint
          .shrink(container.margin)
          .shrink(container.border.width)
          .shrink(container.padding);

        let child_size = child.layout(child_constraint, ctx).size;
        let self_size = match behavior {
          AdaptChildSelfBehavior::Max => constraint
            .max
            .inset_boundary(&container.margin)
            .inset_boundary(&container.border.width),
          AdaptChildSelfBehavior::Child => child_size,
        };
        (self_size, child_size)
      }
    }
  }
}

#[derive(Default)]
pub struct ContainerAlignment {
  pub horizon: HorizontalAlignment,
  pub vertical: VerticalAlignment,
}

impl ContainerAlignment {
  pub fn make_offset(&self, parent: UISize, child: UISize) -> ContainerItemOffset {
    let width_diff = parent.width - child.width;
    let x = match self.horizon {
      HorizontalAlignment::Center => width_diff / 2.,
      HorizontalAlignment::Left => 0.,
      HorizontalAlignment::Right => width_diff,
    };

    let height_diff = parent.height - child.height;
    let y = match self.vertical {
      VerticalAlignment::Center => height_diff / 2.,
      VerticalAlignment::Top => 0.,
      VerticalAlignment::Bottom => height_diff,
    };

    ContainerItemOffset { x, y }
  }
}

#[derive(Default)]
pub struct ContainerItemOffset {
  pub x: f32,
  pub y: f32,
}

trivial_stream_impl!(Container);

impl View for Container {
  fn request(&mut self, detail: &mut ViewRequest) {
    match detail {
      ViewRequest::Event(_) => {}
      ViewRequest::Layout(p) => match p {
        LayoutProtocol::DoLayout {
          constraint, output, ..
        } => {
          self.layout.size = self.size.compute_size_self(*constraint);
          **output = self.layout.size.with_default_baseline();
        }
        LayoutProtocol::PositionAt(p) => self.layout.set_relative_position(*p),
      },
      ViewRequest::Encode(builder) => {
        self
          .layout
          .update_world(builder.current_absolution_origin());
        if self.color.a != 0. {
          builder.present.primitives.push(Primitive::Quad((
            self.layout.into_quad(),
            Style::SolidColor(self.color),
          )));
        }
      }
      ViewRequest::HitTest { point, result } => {
        **result = self.layout.into_quad().is_point_in(*point)
      }
    }
  }
}
