use crate::*;

#[cfg(feature = "serde")]
impl<T, const N: usize, const M: usize> Serialize for Matrix<T, { N }, { M }>
where
  Vector<T, { N }>: Serialize,
{
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut seq = serializer.serialize_tuple(M)?;
    for i in 0..M {
      seq.serialize_element(&self.0[i])?;
    }
    seq.end()
  }
}

#[cfg(feature = "serde")]
impl<'de, T, const N: usize, const M: usize> Deserialize<'de> for Matrix<T, { N }, { M }>
where
  T: Deserialize<'de>,
{
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer
      .deserialize_tuple(N, ArrayVisitor::<[Vector<T, { N }>; M]>::new())
      .map(Matrix)
  }
}
