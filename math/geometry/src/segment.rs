use rendiation_algebra::Scalar;

pub struct Segment<V, S> {
  start: V,
  seg: S,
  end: V,
}

pub struct SegmentChain<V, S> {
  seg: S,
  end: V,
}

pub struct Path<V, S> {
  start: V,
  segments: Vec<SegmentChain<V, S>>,
}

pub trait SpaceLineSegment2<T: Scalar, V> {
  fn sample(&self, t: T, start: &V, end: &V) -> V;
}
