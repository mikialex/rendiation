use crate::*;

/// setup a sized box and use this for positioning child
pub struct Container {
  pub size: LayoutSource<LayoutSize>,
  pub color: Color,
  pub child_align: ContainerAlignment,
  pub child_offset: ContainerItemOffset,
  layout: LayoutUnit,
}

impl Container {
  pub fn size(size: impl Into<LayoutSize>) -> Self {
    Self {
      size: LayoutSource::new(size.into()),
      color: (1., 1., 1., 0.).into(),
      child_align: Default::default(),
      child_offset: Default::default(),
      layout: Default::default(),
    }
  }

  pub fn color(mut self, color: Color) -> Self {
    self.color = color;
    self
  }
}

impl<T> Component<T> for Container {
  fn update(&mut self, _model: &T, ctx: &mut UpdateCtx) {
    self.layout.check_attach(ctx); // this is useless todo
    self.size.refresh(&mut self.layout, ctx);
  }
}

impl<T, C: Component<T>> ComponentAbility<T, C> for Container {
  fn update(&mut self, model: &T, inner: &mut C, ctx: &mut UpdateCtx) {
    self.layout.check_attach(ctx); // this is useless todo
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

#[derive(Default)]
pub struct ContainerAlignment {
  pub horizon: HorizontalAlignment,
  pub vertical: VerticalAlignment,
}

impl ContainerAlignment {
  pub fn make_offset(&self, parent: LayoutSize, child: LayoutSize) -> ContainerItemOffset {
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
    let child_size = inner
      .layout(LayoutConstraint::from_max(*self.size.get()), ctx)
      .size;
    self.layout.size = constraint.clamp(*self.size.get());

    let align_offset = self.child_align.make_offset(self.layout.size, child_size);

    inner.set_position(UIPosition {
      x: align_offset.x + self.child_offset.x,
      y: align_offset.y + self.child_offset.y,
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
    self.layout.size = constraint.clamp(*self.size.get());
    self.layout.size.with_default_baseline()
  }

  fn set_position(&mut self, position: UIPosition) {
    self.layout.set_relative_position(position);
  }
}

impl Presentable for Container {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    self.layout.update_world(builder.current_origin_offset);
    if self.color.a != 0. {
      builder.present.primitives.push(Primitive::Quad((
        self.layout.into_quad(),
        Style::SolidColor(self.color),
      )));
    }
  }
}
