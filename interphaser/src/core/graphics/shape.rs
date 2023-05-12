use crate::*;

impl From<QuadBoundaryWidth> for UISize {
  fn from(v: QuadBoundaryWidth) -> Self {
    (v.left + v.right, v.top + v.bottom).into()
  }
}

impl UISize {
  pub fn inset_boundary(self, b: &QuadBoundaryWidth) -> Self {
    (
      (self.width - b.left - b.right).max(0.),
      (self.height - b.top - b.bottom).max(0.),
    )
      .into()
  }
}

#[derive(Default)]
pub struct QuadBorder {
  pub radius: QuadRadius,
  pub width: QuadBoundaryWidth,
}
