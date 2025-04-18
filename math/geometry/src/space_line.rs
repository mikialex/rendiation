use crate::*;

#[repr(C)]
#[derive(Serialize, Deserialize)]
#[derive(Clone, Copy, Hash, PartialEq, Eq, Facet)]
pub struct SpaceLineSegment<U, X> {
  pub start: U,
  pub end: U,
  pub shape: X,
}

impl<V, X> SpaceLineSegment<V, X> {
  pub fn sample<T>(&self, t: T) -> V
  where
    T: Scalar,
    X: SpaceLineSegmentShape<T, V>,
  {
    self.shape.sample(t, &self.start, &self.end)
  }

  pub fn tangent_at<T>(&self, t: T) -> NormalizedVector<T, V>
  where
    T: Scalar,
    X: SpaceLineSegmentShape<T, V>,
    V: VectorSpace<T> + IntoNormalizedVector<T, V>,
  {
    self.shape.tangent_at(t, &self.start, &self.end)
  }
}

impl<T, U, V, M, X, const D: usize> SpaceEntity<T, D> for SpaceLineSegment<U, X>
where
  T: Scalar,
  M: SquareMatrixDimension<D>,
  V: SpaceEntity<T, D, Matrix = M>,
  U: Positioned<Position = V>,
  X: SpaceEntity<T, D, Matrix = M>,
{
  type Matrix = M;
  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self {
    self.start.mut_position().apply_matrix(mat);
    self.end.mut_position().apply_matrix(mat);
    self.shape.apply_matrix(mat);
    self
  }
}

pub trait SpaceLineSegmentShape<T: Scalar, V> {
  fn sample(&self, t: T, start: &V, end: &V) -> V;

  fn tangent_at(&self, t: T, start: &V, end: &V) -> NormalizedVector<T, V>
  where
    V: VectorSpace<T> + IntoNormalizedVector<T, V>,
  {
    let delta = T::eval::<{ scalar_transmute(0.00001) }>();
    let t1 = (t - delta).max(T::zero());
    let t2 = (t + delta).min(T::one());

    let pt1 = self.sample(t1, start, end);
    let pt2 = self.sample(t2, start, end);

    (pt2 - pt1).into_normalized()
  }
}

pub fn iter_points_by_equally_sampled<T, V, S>(
  curve: &SpaceLineSegment<V, S>,
  divisions: usize,
) -> impl Iterator<Item = V> + '_
where
  T: Scalar,
  S: SpaceLineSegmentShape<T, V>,
{
  assert!(divisions >= 2);
  (0..divisions).map(move |s| curve.sample(T::by_usize_div(s, divisions - 1)))
}
