use crate::*;

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
    (end.position() - start.position()).into_normalized()
  }
}

pub type LineSegment<U> = SpaceLineSegment<U, StraitLine<U>>;
pub type LineSegment2D<T> = LineSegment<Vec2<T>>;

impl<V> LineSegment<V> {
  pub fn line_segment(start: V, end: V) -> Self {
    Self {
      start,
      end,
      shape: StraitLine::default(),
    }
  }

  pub fn iter_point(&self) -> impl Iterator<Item = &V> {
    once(&self.start).chain(once(&self.end))
  }
}

impl<V> LineSegment<V> {
  pub fn map<U>(self, mut f: impl FnMut(V) -> U) -> LineSegment<U> {
    LineSegment {
      start: f(self.start),
      end: f(self.end),
      shape: StraitLine::default(),
    }
  }

  pub fn filter_map<U>(self, mut f: impl FnMut(V) -> Option<U>) -> Option<LineSegment<U>> {
    LineSegment {
      start: f(self.start)?,
      end: f(self.end)?,
      shape: StraitLine::default(),
    }
    .into()
  }

  #[must_use]
  pub fn swap(self) -> Self {
    Self::line_segment(self.end, self.start)
  }

  #[must_use]
  pub fn swap_if(self, prediction: impl FnOnce(Self) -> bool) -> Self
  where
    Self: Copy,
  {
    if prediction(self) {
      self.swap()
    } else {
      self
    }
  }
}
