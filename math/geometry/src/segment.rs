use rendiation_algebra::Scalar;

pub struct Segment<V, S> {
  pub start: V,
  pub seg: S,
  pub end: V,
}

pub struct SegmentChain<V, S> {
  pub seg: S,
  pub end: V,
}

pub struct Path<V, S> {
  pub start: V,
  pub segments: Vec<SegmentChain<V, S>>,
}

pub trait SpaceLineSegment2<T: Scalar, V> {
  fn sample(&self, t: T, start: &V, end: &V) -> V;
}
