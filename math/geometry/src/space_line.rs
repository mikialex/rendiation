use rendiation_algebra::{IntoNormalizedVector, NormalizedVector, Scalar, VectorSpace};

pub trait SpaceLineSegment<T: Scalar, V> {
  fn start(&self) -> V;
  fn end(&self) -> V;
  fn sample(&self, t: T) -> V;

  fn tangent_at(&self, t: T) -> NormalizedVector<T, V>
  where
    V: VectorSpace<T> + IntoNormalizedVector<T, V>,
  {
    let delta = T::eval::<0.00001>();
    let t1 = (t - delta).max(T::zero());
    let t2 = t + delta.min(T::one());

    let pt1 = self.sample(t1);
    let pt2 = self.sample(t2);

    (pt2 - pt1).into_normalized()
  }
}

pub fn iter_points_by_equally_sampled<T: Scalar, V>(
  curve: &impl SpaceLineSegment<T, V>,
  divisions: usize,
) -> impl Iterator<Item = V> + '_ {
  assert!(divisions >= 2);
  (0..divisions)
    .into_iter()
    .map(move |s| curve.sample(T::by_usize_div(s, divisions - 1)))
}
