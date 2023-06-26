use crate::*;

impl From<RectBoundaryWidth> for UISize {
  fn from(v: RectBoundaryWidth) -> Self {
    (v.left + v.right, v.top + v.bottom).into()
  }
}

impl UISize {
  pub fn inset_boundary(self, b: &RectBoundaryWidth) -> Self {
    (
      (self.width - b.left - b.right).max(0.),
      (self.height - b.top - b.bottom).max(0.),
    )
      .into()
  }
}
