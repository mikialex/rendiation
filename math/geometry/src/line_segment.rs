use std::marker::PhantomData;

use crate::{Positioned, SpaceLineSegment, SpaceLineSegmentShape};
use rendiation_algebra::*;

#[derive(Copy, Clone, PartialEq, Eq, std::hash::Hash)]
pub struct StraitLine<U> {
  phantom: PhantomData<U>,
}

impl<U> Default for StraitLine<U> {
  fn default() -> Self {
    Self {
      phantom: Default::default(),
    }
  }
}

impl<T, U, V, M, const D: usize> SpaceEntity<T, D> for StraitLine<U>
where
  T: Scalar,
  M: SquareMatrixDimension<D>,
  V: SpaceEntity<T, D, Matrix = M>,
  U: Positioned<Position = V>,
{
  type Matrix = M;
  fn apply_matrix(&mut self, _mat: Self::Matrix) -> &mut Self {
    self
  }
}

impl<T, V> SpaceLineSegmentShape<T, V> for StraitLine<V>
where
  T: Scalar,
  V: Positioned<Position = V>,
  V: Lerp<T> + Copy,
{
  fn sample(&self, t: T, start: &V, end: &V) -> V {
    start.lerp(*end, t)
  }
  fn tangent_at(&self, _t: T, start: &V, end: &V) -> NormalizedVector<T, V>
  where
    V: VectorSpace<T> + IntoNormalizedVector<T, V>,
  {
    (*end.position() - *start.position()).into_normalized()
  }
}

pub type LineSegment<U> = SpaceLineSegment<U, StraitLine<U>>;

impl<V> LineSegment<V> {
  pub fn line_segment(start: V, end: V) -> Self {
    Self {
      start,
      end,
      shape: StraitLine::default(),
    }
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
      shape: StraitLine::default(),
    }
  }

  pub fn swap(&self) -> Self {
    Self::line_segment(self.end, self.start)
  }

  pub fn swap_if(&self, prediction: impl FnOnce(Self) -> bool) -> Self {
    if prediction(*self) {
      self.swap()
    } else {
      *self
    }
  }
}
