use crate::*;

impl<T, const N: usize> Serialize for Vector<T, { N }>
where
  T: Serialize,
{
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut seq = serializer.serialize_tuple(N)?;
    for i in 0..N {
      seq.serialize_element(&self.0[i])?;
    }
    seq.end()
  }
}

impl<'de, T, const N: usize> Deserialize<'de> for Vector<T, { N }>
where
  T: Deserialize<'de>,
{
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer
      .deserialize_tuple(N, ArrayVisitor::<[T; N]>::new())
      .map(Vector)
  }
}
