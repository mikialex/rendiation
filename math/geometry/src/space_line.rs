use rendiation_algebra::{IntoNormalizedVector, NormalizedVector, Scalar, VectorSpace};

use crate::SpaceLineSegment;

pub trait SpaceLineSegmentShape<T: Scalar, V> {
  fn sample(&self, t: T, start: &V, end: &V) -> V;

  fn tangent_at(&self, t: T, start: &V, end: &V) -> NormalizedVector<T, V>
  where
    V: VectorSpace<T> + IntoNormalizedVector<T, V>,
  {
    let delta = T::eval::<0.00001>();
    let t1 = (t - delta).max(T::zero());
    let t2 = t + delta.min(T::one());

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
  (0..divisions)
    .into_iter()
    .map(move |s| curve.sample(T::by_usize_div(s, divisions - 1)))
}
