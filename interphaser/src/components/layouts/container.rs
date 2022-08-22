use crate::*;

/// setup a sized box and use this for positioning child
pub struct Container {
  pub color: Color,
  pub child_align: ContainerAlignment,
  /// extra relative offset
  pub child_offset: ContainerItemOffset,
  pub size: LayoutSource<ContainerSize>,
  /// for simplicity, we only support outer border now
  pub border: QuadBorder,
  pub margin: QuadBoundaryWidth,
  pub padding: QuadBoundaryWidth,
  layout: LayoutUnit,
}

impl Container {
  pub fn sized(size: impl Into<UISize<UILength>>) -> Self {
    Self::size(ContainerSize::ConstraintChild { size: size.into() })
  }

  pub fn adapt(behavior: AdaptChildSelfBehavior) -> Self {
    Self::size(ContainerSize::AdaptChild { behavior })
  }

  pub fn size(size: ContainerSize) -> Self {
    Self {
      color: (1., 1., 1., 0.).into(),
      child_align: Default::default(),
      child_offset: Default::default(),
      size: LayoutSource::new(size),
      layout: Default::default(),
      border: Default::default(),
      margin: Default::default(),
      padding: Default::default(),
    }
  }

  #[must_use]
  pub fn color(mut self, color: Color) -> Self {
    self.color = color;
    self
  }
}

impl<T> Component<T> for Container {
  fn update(&mut self, _model: &T, ctx: &mut UpdateCtx) {
    self.size.refresh(&mut self.layout, ctx);
  }
}

impl<T, C: Component<T>> ComponentAbility<T, C> for Container {
  fn update(&mut self, model: &T, inner: &mut C, ctx: &mut UpdateCtx) {
    self.size.refresh(&mut self.layout, ctx);
    inner.update(model, ctx);
    self.layout.or_layout_change(ctx);
  }

  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {
    inner.event(model, event);
  }
}

impl<C: Presentable> PresentableAbility<C> for Container {
  fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C) {
    Presentable::render(self, builder);
    builder.push_offset(self.layout.relative_position);
    inner.render(builder);
    builder.pop_offset()
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

  /// (self size, child size)
  pub fn compute_size_pair(
    &self,
    constraint: LayoutConstraint,
    container: &Container,
    child: &mut dyn LayoutAble,
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

impl<C: LayoutAble> LayoutAbility<C> for Container {
  fn layout(
    &mut self,
    constraint: LayoutConstraint,
    ctx: &mut LayoutCtx,
    inner: &mut C,
  ) -> LayoutResult {
    if self.layout.skipable(constraint) {
      return self.layout.size.with_default_baseline();
    }

    let (self_size, child_size) = self
      .size
      .get()
      .compute_size_pair(constraint, self, inner, ctx);

    self.layout.size = self_size;

    let align_offset = self.child_align.make_offset(self.layout.size, child_size);

    inner.set_position(UIPosition {
      x: align_offset.x + self.child_offset.x + self.margin.left + self.padding.left  + self.border.width.left,
      y: align_offset.y + self.child_offset.y + self.margin.top + self.padding.top  + self.border.width.top,
    });

    self.layout.size.with_default_baseline()
  }

  fn set_position(&mut self, position: UIPosition, _inner: &mut C) {
    self.layout.set_relative_position(position);
  }
}

impl<C> HotAreaPassBehavior<C> for Container {
  fn is_point_in(&self, point: crate::UIPosition, _inner: &C) -> bool {
    self.layout.into_quad().is_point_in(point)
  }
}

impl LayoutAble for Container {
  fn layout(&mut self, constraint: LayoutConstraint, _ctx: &mut LayoutCtx) -> LayoutResult {
    self.layout.size = self.size.get().compute_size_self(constraint);
    self.layout.size.with_default_baseline()
  }

  fn set_position(&mut self, position: UIPosition) {
    self.layout.set_relative_position(position);
  }
}

impl Presentable for Container {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    self.layout.update_world(builder.current_origin_offset());
    if self.color.a != 0. {
      builder.present.primitives.push(Primitive::Quad((
        self.layout.into_quad(),
        Style::SolidColor(self.color),
      )));
    }
  }
}
