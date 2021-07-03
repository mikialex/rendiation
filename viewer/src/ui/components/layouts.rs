use crate::ui::*;

#[derive(PartialEq, Clone)]
pub struct Container {
  margin: EdgeInsets,
  border: EdgeInsets,
  padding: EdgeInsets,
  width: Option<f32>,
  height: Option<f32>,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum UIDirection {
  Horizon,
  Vertical,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct EdgeInsets {
  left: f32,
  right: f32,
  top: f32,
  bottom: f32,
}

impl Default for EdgeInsets {
  fn default() -> Self {
    Self::all(0.)
  }
}

impl EdgeInsets {
  pub fn symmetric(direction: UIDirection, value: f32) -> Self {
    match direction {
      UIDirection::Horizon => Self::from_l_t_r_b(value, value, 0., 0.),
      UIDirection::Vertical => Self::from_l_t_r_b(0., 0., value, value),
    }
  }
  pub fn all(value: f32) -> Self {
    Self {
      left: value,
      right: value,
      top: value,
      bottom: value,
    }
  }
  pub fn from_l_t_r_b(left: f32, right: f32, top: f32, bottom: f32) -> Self {
    Self {
      left,
      right,
      top,
      bottom,
    }
  }
}

impl LayoutConstraint {
  pub fn consume_by_edge(&self, edge: EdgeInsets) -> Self {
    Self {
      width_min: self.width_min - (edge.left + edge.right),
      width_max: self.width_max - (edge.left + edge.right),
      height_min: self.height_min - (edge.top + edge.bottom),
      height_max: self.height_max - (edge.top + edge.bottom),
    }
    .min_zero()
  }
}

impl Default for Container {
  fn default() -> Self {
    Self {
      margin: Default::default(),
      border: Default::default(),
      padding: Default::default(),
      width: None,
      height: None,
    }
  }
}

pub fn layout_children_one_by_one_vertically(ctx: &mut LayoutCtx) -> LayoutSize {
  let mut current_y = ctx.self_position.y;
  let mut max_width: f32 = 0.;
  let mut constraint = ctx.parent_constraint;
  ctx.children.iter_mut().for_each(|c| {
    let size = c.layout(constraint);
    c.set_position(UIPosition {
      x: ctx.self_position.x,
      y: current_y,
    });
    current_y += size.height;
    max_width = max_width.max(size.width);
    constraint = constraint.consume_height(size.height);
  });
  LayoutSize {
    width: max_width,
    height: current_y - ctx.self_position.y,
  }
}

impl Component for Container {
  type State = ();
  fn layout(&self, state: &Self::State, ctx: &mut LayoutCtx) -> LayoutSize {
    let mut children_constraint = ctx
      .parent_constraint
      .consume_by_edge(self.margin)
      .consume_by_edge(self.border)
      .consume_by_edge(self.padding);

    if let Some(width) = self.width {
      children_constraint.set_max_width(width);
    }
    if let Some(height) = self.height {
      children_constraint.set_max_height(height);
    }

    let children_start = ctx
      .self_position
      .move_into_top_left_corner(self.margin)
      .move_into_top_left_corner(self.border)
      .move_into_top_left_corner(self.padding);

    ctx.parent_constraint = children_constraint;
    ctx.self_position = children_start;
    layout_children_one_by_one_vertically(ctx)
  }
}

impl UIPosition {
  pub fn move_into_top_left_corner(&self, edge: EdgeInsets) -> Self {
    Self{
        x: self.x + edge.left,
        y: self.y + edge.top,
    }
  }
}

#[derive(Default, PartialEq, Clone)]
pub struct Row;

impl Component for Row {
  type State = ();
  // fn layout(&self, state: &Self::State, ctx: &mut LayoutCtx) -> LayoutSize {}
}
