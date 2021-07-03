use super::{Component, ComponentCell};

pub trait LayoutAble {
  fn layout(&mut self, constraint: LayoutConstraint) -> LayoutSize;
  fn set_position(&mut self, position: UIPosition);
}

impl<T, P> LayoutAble for ComponentCell<T, P>
where
  T: Component,
  P: Component,
{
  fn layout(&mut self, constraint: LayoutConstraint) -> LayoutSize {
    let children = if self.meta.children.is_empty() {
      &mut self.meta.out_children
    } else {
      &mut self.meta.children
    };
    let mut children: Vec<_> = children.iter_mut().map(|c| c.as_layout()).collect();

    let mut ctx = LayoutCtx {
      parent_constraint: constraint,
      self_position: self.meta.layout.position,
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
  pub parent_constraint: LayoutConstraint,
  pub self_position: UIPosition,
  pub children: &'a mut [&'a mut dyn LayoutAble],
}

#[derive(Debug, Clone, Copy)]
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
  pub fn from_max(size: LayoutSize) -> Self {
    Self {
      width_min: 0.,
      width_max: 0.,
      height_min: size.width,
      height_max: size.height,
    }
  }
  pub fn max(&self) -> LayoutSize {
    LayoutSize {
      width: self.width_max,
      height: self.height_max,
    }
  }

  pub fn set_max_width(&mut self, width: f32) {
    self.width_max = width;
    self.width_max = self.width_max.max(self.width_min);
  }

  pub fn set_max_height(&mut self, height: f32) {
    self.height_max = height;
    self.height_max = self.height_max.max(self.height_min);
  }

  pub fn consume_width(&self, width: f32) -> Self {
    Self {
      width_min: self.width_min - width,
      width_max: self.width_max - width,
      ..*self
    }
    .min_zero()
  }

  pub fn consume_height(&self, height: f32) -> Self {
    Self {
      height_min: self.height_min - height,
      height_max: self.height_max - height,
      ..*self
    }
    .min_zero()
  }

  pub fn min_zero(&self) -> Self {
    Self {
      width_min: self.width_min.min(0.),
      width_max: self.width_max.min(0.),
      height_min: self.height_min.min(0.),
      height_max: self.height_max.min(0.),
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
