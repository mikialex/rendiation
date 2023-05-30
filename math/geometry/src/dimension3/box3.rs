use crate::*;

pub type Box3<T = f32> = HyperAABB<Vec3<T>>;

impl<T: Scalar> LebesgueMeasurable<T, 2> for Box3<T> {
  #[inline(always)]
  fn measure(&self) -> T {
    T::two()
      * (self.width() * self.height() + self.width() * self.depth() + self.height() * self.depth())
  }
}

impl<T: Scalar> LebesgueMeasurable<T, 3> for Box3<T> {
  #[inline(always)]
  fn measure(&self) -> T {
    self.width() * self.height() * self.depth()
  }
}

impl<T: Scalar> SpaceEntity<T, 3> for Box3<T> {
  type Matrix = Mat4<T>;
  fn apply_matrix(&mut self, m: Self::Matrix) -> &mut Self {
    if self.is_empty() {
      return self;
    }
    let points = [
      *Vec3::new(self.min.x, self.min.y, self.min.z).apply_matrix(m), // 000
      *Vec3::new(self.min.x, self.min.y, self.max.z).apply_matrix(m), // 001
      *Vec3::new(self.min.x, self.max.y, self.min.z).apply_matrix(m), // 010
      *Vec3::new(self.min.x, self.max.y, self.max.z).apply_matrix(m), // 011
      *Vec3::new(self.max.x, self.min.y, self.min.z).apply_matrix(m), // 100
      *Vec3::new(self.max.x, self.min.y, self.max.z).apply_matrix(m), // 101
      *Vec3::new(self.max.x, self.max.y, self.min.z).apply_matrix(m), // 110
      *Vec3::new(self.max.x, self.max.y, self.max.z).apply_matrix(m), // 111
    ];
    *self = points.iter().collect();
    self
  }
}

impl Default for Box3 {
  fn default() -> Self {
    Self::empty()
  }
}

impl<T: Scalar> Box3<T> {
  pub fn new3(min: Vec3<T>, max: Vec3<T>) -> Self {
    Self { min, max }
  }

  #[inline(always)]
  pub fn new_cube(center: Vec3<T>, radius: T) -> Self {
    Self::new_from_center(center, Vec3::splat(radius))
  }

  #[inline(always)]
  pub fn new_from_center(center: Vec3<T>, half_size: Vec3<T>) -> Self {
    Self {
      min: center - half_size,
      max: center + half_size,
    }
  }

  #[inline(always)]
  pub fn size(&self) -> Vec3<T> {
    Vec3::new(self.width(), self.height(), self.depth())
  }

  #[inline(always)]
  pub fn half_size(&self) -> Vec3<T> {
    self.size() * T::half()
  }

  #[inline(always)]
  pub fn width(&self) -> T {
    self.max.x - self.min.x
  }

  #[inline(always)]
  pub fn height(&self) -> T {
    self.max.y - self.min.y
  }

  #[inline(always)]
  pub fn depth(&self) -> T {
    self.max.z - self.min.z
  }

  #[inline(always)]
  pub fn center(&self) -> Vec3<T> {
    (self.min + self.max) * T::half()
  }

  #[rustfmt::skip]
  #[inline(always)]
  pub fn max_corner(&self, direction: Vec3<T>) -> Vec3<T> {
    Vec3::new(
      if direction.x > T::zero() { self.max.x } else { self.min.x },
      if direction.y > T::zero() { self.max.y } else { self.min.y },
      if direction.z > T::zero() { self.max.z } else { self.min.z },
    )
  }

  #[inline(always)]
  pub fn longest_axis(&self) -> (Axis3, T) {
    let x_length = self.max.x - self.min.x;
    let y_length = self.max.y - self.min.y;
    let z_length = self.max.z - self.min.z;

    if x_length > y_length {
      if x_length > z_length {
        (Axis3::X, x_length)
      } else {
        (Axis3::Z, z_length)
      }
    } else if y_length > z_length {
      (Axis3::Y, y_length)
    } else {
      (Axis3::Z, z_length)
    }
  }

  #[inline(always)]
  pub fn is_empty(&self) -> bool {
    (self.max.x < self.min.x) || (self.max.y < self.min.y) || (self.max.z < self.min.z)
  }
}

impl<'a, T: Scalar> FromIterator<&'a Vec3<T>> for Box3<T> {
  fn from_iter<I: IntoIterator<Item = &'a Vec3<T>>>(items: I) -> Self {
    let mut bbox = Self::empty();
    items.into_iter().for_each(|p| bbox.expand_by_point(*p));
    bbox
  }
}

impl<T: Scalar> FromIterator<Vec3<T>> for Box3<T> {
  fn from_iter<I: IntoIterator<Item = Vec3<T>>>(items: I) -> Self {
    let mut bbox = Self::empty();
    items.into_iter().for_each(|p| bbox.expand_by_point(p));
    bbox
  }
}

impl<'a, T: Scalar> FromIterator<&'a Box3<T>> for Box3<T> {
  fn from_iter<I: IntoIterator<Item = &'a Box3<T>>>(items: I) -> Self {
    let mut bbox = Self::empty();
    items.into_iter().for_each(|p| bbox.expand_by_other(*p));
    bbox
  }
}

impl<T: Scalar> FromIterator<Box3<T>> for Box3<T> {
  fn from_iter<I: IntoIterator<Item = Box3<T>>>(items: I) -> Self {
    let mut bbox = Self::empty();
    items.into_iter().for_each(|p| bbox.expand_by_other(p));
    bbox
  }
}

impl<T: Scalar> ContainAble<T, Box3<T>, 3> for Box3<T> {
  fn contains(&self, box3: &Box3<T>) -> bool {
    self.min.x <= box3.min.x
      && self.min.y <= box3.min.y
      && self.min.z <= box3.min.z
      && self.max.x >= box3.max.x
      && self.max.y >= box3.max.y
      && self.max.z >= box3.max.z
  }
}

impl<T: Scalar> Box3<T> {
  /// return (near, far)
  pub fn get_near_far_by_direction(&self, dir: NormalizedVector<T, Vec3<T>>) -> (Vec3<T>, Vec3<T>) {
    let (mut far, mut near) = (self.min, self.max);

    if dir.x >= T::zero() {
      far.x = self.max.x;
      near.x = self.min.x;
    }

    if dir.y >= T::zero() {
      far.y = self.max.y;
      near.y = self.min.y;
    }

    if dir.z >= T::zero() {
      far.z = self.max.z;
      near.z = self.min.z;
    }

    (far, near)
  }
}
