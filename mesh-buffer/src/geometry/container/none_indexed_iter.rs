use super::super::*;
use core::marker::PhantomData;
use rendiation_math_entity::*;

pub struct PrimitiveIter<'a, V: Positioned3D, T: PrimitiveData<V>> {
  data: &'a [V],
  current: usize,
  _phantom: PhantomData<T>,
}

impl<'a, V: Positioned3D, T: PrimitiveData<V>> ExactSizeIterator for PrimitiveIter<'a, V, T> {
  fn len(&self) -> usize {
    self.data.len() / T::DATA_STRIDE - self.current
  }
}

impl<'a, V: Positioned3D, T: PrimitiveData<V>> PrimitiveIter<'a, V, T> {
  pub fn new(data: &'a [V]) -> Self {
    Self {
      data,
      current: 0,
      _phantom: PhantomData,
    }
  }
}

impl<'a, V: Positioned3D, T: PrimitiveData<V>> Iterator for PrimitiveIter<'a, V, T> {
  type Item = T;

  fn next(&mut self) -> Option<T> {
    self.current += 1;
    if self.current == self.data.len() - 1 {
      None
    } else {
      Some(T::from_data(self.data, self.current as usize))
    }
  }
}
