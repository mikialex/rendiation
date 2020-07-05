
// https://github.com/maplant/aljabar/blob/master/src/lib.rs
pub trait MetricSpace: Sized {
  type Metric;

  /// Returns the distance squared between the two values.
  fn distance(self, other: Self) -> Self::Metric;
}

/// Vector spaces that have an inner (also known as "dot") product.
pub trait InnerSpace: VectorSpace
where
    Self: Clone,
    Self: MetricSpace<Metric = <Self as VectorSpace>::Scalar>,
{
    /// Return the inner (also known as "dot") product.
    fn dot(self, other: Self) -> Self::Scalar;

    /// Returns the squared length of the value.
    fn magnitude2(self) -> Self::Scalar {
        self.clone().dot(self)
    }

    /// Returns the [reflection](https://en.wikipedia.org/wiki/Reflection_(mathematics))
    /// of the current vector with respect to the given surface normal. The
    /// surface normal must be of length 1 for the return value to be
    /// correct. The current vector is interpreted as pointing toward the
    /// surface, and does not need to be normalized.
    fn reflect(self, surface_normal: Self) -> Self {
        let a = surface_normal.clone() * self.clone().dot(surface_normal);
        self - (a.clone() + a)
    }
}