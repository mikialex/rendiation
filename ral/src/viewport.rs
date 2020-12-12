#[derive(Copy, Clone)]
pub struct Viewport {
  pub x: f32,
  pub y: f32,
  pub w: f32,
  pub h: f32,
  pub min_depth: f32,
  pub max_depth: f32,
}

impl Viewport {
  pub fn new(size: (usize, usize)) -> Self {
    Self {
      x: 0.0,
      y: 0.0,
      w: size.0 as f32,
      h: size.1 as f32,
      min_depth: 0.0,
      max_depth: 1.0,
    }
  }

  #[allow(clippy::float_cmp)]
  pub fn is_depth_default(&self) -> bool {
    self.min_depth == 0.0 && self.max_depth == 1.0
  }

  pub fn set_size(&mut self, w: f32, h: f32) {
    self.w = w;
    self.h = h;
  }
}
