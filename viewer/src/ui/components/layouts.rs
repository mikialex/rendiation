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

impl Component for Container {
  type State = ();
  fn layout(&self, state: &Self::State, ctx: &mut LayoutCtx) -> LayoutSize {
    // let mut children_constraint = ctx.parent_constraint.
    todo!()
  }
}

#[derive(Default, PartialEq, Clone)]
pub struct Row;

impl Component for Row {
  type State = ();
  // fn layout(&self, state: &Self::State, ctx: &mut LayoutCtx) -> LayoutSize {}
}
