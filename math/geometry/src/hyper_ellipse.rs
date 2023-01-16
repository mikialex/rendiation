#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct HyperEllipse<V> {
  pub center: V,
  pub radius: V,
}
