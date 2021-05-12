use crate::{SpaceEntity, SpaceLineSegment};
use rendiation_algebra::{Lerp, Scalar, SquareMatrixType};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct LineSegment<U> {
  pub start: U,
  pub end: U,
}

impl<T, U, const D: usize, V> SpaceEntity<T, D> for LineSegment<U>
where
  T: Scalar,
  V: SpaceEntity<T, D>,
  U: DerefMut<Target = V>,
{
  fn apply_matrix(&mut self, mat: SquareMatrixType<T, D>) -> &mut Self {
    self.start.deref_mut().apply_matrix(mat);
    self.end.deref_mut().apply_matrix(mat);
    self
  }
}

impl<T, U, V> SpaceLineSegment<T, V> for LineSegment<U>
where
  T: Scalar,
  U: Deref<Target = V> + Copy,
  V: Lerp<T> + Copy,
{
  fn start(&self) -> V {
    *self.start
  }
  fn end(&self) -> V {
    *self.end
  }
  fn sample(&self, t: T) -> V {
    self.start().lerp(self.end(), t)
  }
}

impl<V> LineSegment<V> {
  pub fn new(start: V, end: V) -> Self {
    Self { start, end }
  }

  pub fn iter_point(&self) -> LineSegmentIter<'_, V> {
    LineSegmentIter::new(self)
  }
}

pub struct LineSegmentIter<'a, V> {
  line_segment: &'a LineSegment<V>,
  visit_count: i8,
}

impl<'a, V> LineSegmentIter<'a, V> {
  pub fn new(line3: &'a LineSegment<V>) -> Self {
    Self {
      line_segment: line3,
      visit_count: -1,
    }
  }
}

impl<'a, V: Copy> Iterator for LineSegmentIter<'a, V> {
  type Item = V;
  fn next(&mut self) -> Option<Self::Item> {
    self.visit_count += 1;
    if self.visit_count == 0 {
      Some(self.line_segment.start)
    } else if self.visit_count == 1 {
      Some(self.line_segment.end)
    } else {
      None
    }
  }
}

impl<V: Copy> LineSegment<V> {
  pub fn map<U>(&self, f: impl Fn(V) -> U) -> LineSegment<U> {
    LineSegment {
      start: f(self.start),
      end: f(self.end),
    }
  }

  pub fn swap(&self) -> Self {
    Self::new(self.end, self.start)
  }

  pub fn swap_if(&self, prediction: impl FnOnce(Self) -> bool) -> Self {
    if prediction(*self) {
      self.swap()
    } else {
      *self
    }
  }
}
