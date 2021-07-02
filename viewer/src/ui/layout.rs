pub trait LayoutAble {
  fn request_size(&self, constraint: &LayoutConstraint) -> LayoutSize;
  fn update(&mut self) -> &mut Layout;
}

pub struct LayoutCtx<'a> {
  children: [&'a mut dyn LayoutAble],
}

pub struct LayoutConstraint {
  pub width_min: f32,
  pub width_max: f32,
  pub height_min: f32,
  pub height_max: f32,
}

impl LayoutConstraint {
  pub fn max(&self) -> LayoutSize {
    LayoutSize {
      width: self.width_max,
      height: self.height_max,
    }
  }
}

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
