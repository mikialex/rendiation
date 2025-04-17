use crate::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Facet)]
pub struct HyperPlane<T: Scalar, V> {
  /// face normal
  pub normal: NormalizedVector<T, V>,
  /// plane to the origin signed distance
  pub constant: T,
}

impl<T: Scalar, V> HyperPlane<T, V> {
  pub fn new(normal: NormalizedVector<T, V>, constant: T) -> Self {
    Self { normal, constant }
  }

  pub fn from_normal_and_plane_point(normal: V, point: V) -> Self
  where
    V: IntoNormalizedVector<T, V>,
    V: InnerProductSpace<T>,
  {
    let normal = normal.into_normalized();
    let constant = -normal.dot(point);
    Self::new(normal, constant)
  }

  pub fn from_normal_and_origin_point(normal: V) -> Self
  where
    V: IntoNormalizedVector<T, V>,
  {
    let normal = normal.into_normalized();
    Self::new(normal, T::zero())
  }

  pub fn flip(&mut self)
  where
    V: InnerProductSpace<T>,
  {
    self.normal = self.normal.reverse();
    self.constant = -self.constant;
  }
}
