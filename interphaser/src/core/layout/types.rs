use crate::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConstraint {
  pub min: UISize,
  pub max: UISize,
}

impl Default for LayoutConstraint {
  fn default() -> Self {
    Self::UNBOUNDED
  }
}

impl LayoutConstraint {
  /// An unbounded box constraints object.
  ///
  /// Can be satisfied by any nonnegative size.
  pub const UNBOUNDED: Self = Self {
    min: UISize::ZERO,
    max: UISize::new(f32::INFINITY, f32::INFINITY),
  };

  /// Create a new box constraints object.
  ///
  /// Create constraints based on minimum and maximum size.
  ///
  /// The given sizes are also [rounded away from zero],
  /// so that the layout is aligned to integers.
  ///
  /// [rounded away from zero]: struct.Size.html#method.expand
  pub fn new(min: UISize, max: UISize) -> Self {
    Self { min, max }
  }
  /// Create a "tight" box constraints object.
  ///
  /// A "tight" constraint can only be satisfied by a single size.
  ///
  /// The given size is also [rounded away from zero],
  /// so that the layout is aligned to integers.
  ///
  /// [rounded away from zero]: struct.Size.html#method.expand
  pub fn tight(size: UISize) -> Self {
    Self {
      min: size,
      max: size,
    }
  }

  /// Create a "loose" version of the constraints.
  ///
  /// Make a version with zero minimum size, but the same maximum size.
  #[must_use]
  pub fn loosen(&self) -> Self {
    Self {
      min: UISize::ZERO,
      max: self.max,
    }
  }

  /// Clamp a given size so that it fits within the constraints.
  ///
  /// The given size is also [rounded away from zero],
  /// so that the layout is aligned to integers.
  ///
  /// [rounded away from zero]: struct.Size.html#method.expand
  pub fn constrain(&self, size: impl Into<UISize>) -> UISize {
    size.into().clamp(self.min, self.max)
  }

  pub fn from_max(size: UISize) -> Self {
    Self {
      min: UISize::ZERO,
      max: size,
    }
  }
  pub fn max(&self) -> UISize {
    self.max
  }
  pub fn min(&self) -> UISize {
    self.min
  }
  pub fn clamp(&self, size: UISize) -> UISize {
    UISize {
      width: size.width.clamp(self.min.width, self.max.width),
      height: size.height.clamp(self.min.height, self.max.height),
    }
  }

  /// Shrink min and max constraints by size
  ///
  /// The given size is also [rounded away from zero],
  /// so that the layout is aligned to integers.
  ///
  /// [rounded away from zero]: struct.Size.html#method.expand
  #[must_use]
  pub fn shrink(&self, diff: impl Into<UISize>) -> Self {
    let diff = diff.into();
    let min = UISize::new(
      (self.min().width - diff.width).max(0.),
      (self.min().height - diff.height).max(0.),
    );
    let max = UISize::new(
      (self.max().width - diff.width).max(0.),
      (self.max().height - diff.height).max(0.),
    );

    Self::new(min, max)
  }

  /// Test whether these constraints contain the given `Size`.
  pub fn contains(&self, size: impl Into<UISize>) -> bool {
    let size = size.into();
    (self.min.width <= size.width && size.width <= self.max.width)
      && (self.min.height <= size.height && size.height <= self.max.height)
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UILength {
  Px(f32),
  ParentPercent(f32),
}

/// convert float default to logic pixel
impl From<f32> for UILength {
  fn from(v: f32) -> Self {
    Self::Px(v)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UISize<T = f32> {
  pub width: T,
  pub height: T,
}

impl UISize<UILength> {
  pub fn into_pixel(&self, parent: UISize) -> UISize {
    let width = match self.width {
      UILength::Px(px) => px,
      UILength::ParentPercent(p) => parent.width * p / 100.,
    };
    let height = match self.height {
      UILength::Px(px) => px,
      UILength::ParentPercent(p) => parent.height * p / 100.,
    };
    UISize { width, height }
  }
}

impl UISize {
  pub const ZERO: Self = Self {
    width: 0.,
    height: 0.,
  };
  pub const fn new(width: f32, height: f32) -> Self {
    Self { width, height }
  }

  pub fn with_default_baseline(self) -> LayoutResult {
    LayoutResult {
      size: self,
      baseline_offset: 0.,
    }
  }

  #[must_use]
  pub fn clamp(self, min: Self, max: Self) -> Self {
    let width = self.width.clamp(min.width, max.width);
    let height = self.height.clamp(min.height, max.height);
    Self { width, height }
  }
}

impl<X, T: Into<X>> From<(T, T)> for UISize<X> {
  fn from(value: (T, T)) -> Self {
    Self {
      width: value.0.into(),
      height: value.1.into(),
    }
  }
}

impl<T: From<f32>> From<UISize> for (T, T) {
  fn from(value: UISize) -> Self {
    (value.width.into(), value.height.into())
  }
}

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

  pub fn union(self, other: Self) -> Self {
    Self {
      width: self.width.max(other.width),
      height: self.height.max(other.height),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UIPosition {
  pub x: f32,
  pub y: f32,
}

impl From<(f32, f32)> for UIPosition {
  fn from(v: (f32, f32)) -> Self {
    Self { x: v.0, y: v.1 }
  }
}

impl From<Vec2<f32>> for UIPosition {
  fn from(v: Vec2<f32>) -> Self {
    Self { x: v.x, y: v.y }
  }
}

impl From<UIPosition> for Vec2<f32> {
  fn from(val: UIPosition) -> Self {
    Vec2 { x: val.x, y: val.y }
  }
}
