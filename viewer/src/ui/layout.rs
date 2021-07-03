use super::{Component, ComponentCell};

pub trait LayoutAble {
  fn layout(&mut self, constraint: &LayoutConstraint) -> LayoutSize;
  fn set_position(&mut self, position: UIPosition);
}

impl<T, P> LayoutAble for ComponentCell<T, P>
where
  T: Component,
  P: Component,
{
  fn layout(&mut self, constraint: &LayoutConstraint) -> LayoutSize {
    let mut children: Vec<_> = self
      .meta
      .children
      .iter_mut()
      .map(|c| c.as_layout())
      .collect();

    let mut ctx = LayoutCtx {
      self_position: &self.meta.layout.position,
      children: children.as_mut(),
    };
    let size = self.data.props.layout(&self.data.state, &mut ctx);
    self.meta.layout.size = size;
    size
  }

  fn set_position(&mut self, position: UIPosition) {
    self.meta.layout.position = position;
  }
}

pub struct LayoutCtx<'a> {
  pub self_position: &'a UIPosition,
  pub children: &'a mut [&'a mut dyn LayoutAble],
}

pub struct LayoutConstraint {
  pub width_min: f32,
  pub width_max: f32,
  pub height_min: f32,
  pub height_max: f32,
}

impl LayoutConstraint {
  pub fn unlimited() -> Self {
    Self {
      width_min: 0.,
      width_max: 0.,
      height_min: f32::INFINITY,
      height_max: f32::INFINITY,
    }
  }
  pub fn max(&self) -> LayoutSize {
    LayoutSize {
      width: self.width_max,
      height: self.height_max,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutSize {
  pub width: f32,
  pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UIPosition {
  pub x: f32,
  pub y: f32,
}

/// Layout coordinate use x => right. y => down (same as web API canvas2D);
pub struct Layout {
  pub position: UIPosition,
  pub size: LayoutSize,
}

impl Default for Layout {
  fn default() -> Self {
    Self {
      position: UIPosition { x: 0., y: 0. },
      size: LayoutSize {
        width: 0.,
        height: 0.,
      },
    }
  }
}
