#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HyperEllipse<V> {
  pub center: V,
  pub radius: V,
}
