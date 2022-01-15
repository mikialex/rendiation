#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum HorizontalAlignment {
  Center,
  Left,
  Right,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum VerticalAlignment {
  Center,
  Top,
  Bottom,
}

impl Default for HorizontalAlignment {
  fn default() -> Self {
    Self::Center
  }
}

impl Default for VerticalAlignment {
  fn default() -> Self {
    Self::Center
  }
}
