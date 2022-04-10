#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HyperAARange<V> {
  pub origin: V,
  pub extent: V,
}
