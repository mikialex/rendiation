use crate::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Facet)]
pub struct HyperEllipse<V> {
  pub center: V,
  pub radius: V,
}
