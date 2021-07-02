use super::{Component, ComponentCell};

pub trait LayoutAble {
  fn request_size(&self, constraint: &LayoutConstraint) -> LayoutSize;
  fn set_layout(&mut self, layout: Layout);
}

impl<T, P> LayoutAble for ComponentCell<T, P>
where
  T: Component,
  P: Component,
{
  fn request_size(&self, constraint: &LayoutConstraint) -> LayoutSize {
    self
      .data
      .props
      .request_layout_size(&self.data.state, constraint)
  }

  fn set_layout(&mut self, layout: Layout) {
    self.meta.layout = layout;
  }
}

pub struct LayoutCtx<'a> {
  pub self_layout: &'a Layout,
  pub children: [&'a mut dyn LayoutAble],
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

pub struct Layout {
  pub x: f32,
  pub y: f32,
  pub size: LayoutSize,
}

impl Default for Layout {
  fn default() -> Self {
    Self {
      x: 0.,
      y: 0.,
      size: LayoutSize {
        width: 0.,
        height: 0.,
      },
    }
  }
}
