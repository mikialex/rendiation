use super::super::*;
use super::*;
use core::marker::PhantomData;

pub struct IndexedPrimitiveIter<'a, V: Positioned3D, T: PrimitiveData<V>> {
  index: &'a [u16],
  data: &'a [V],
  current: usize,
  _phantom: PhantomData<T>,
}

impl<'a, V: Positioned3D, T: PrimitiveData<V>> IndexedPrimitiveIter<'a, V, T> {
  pub fn new(index: &'a [u16], data: &'a [V]) -> Self {
    Self {
      index,
      data,
      current: 0,
      _phantom: PhantomData,
    }
  }
}

impl<'a, V: Positioned3D, T: PrimitiveData<V>> Iterator for IndexedPrimitiveIter<'a, V, T> {
  type Item = (T, T::IndexIndicator);

  fn next(&mut self) -> Option<(T, T::IndexIndicator)> {
    self.current += 1;
    if self.current == self.index.len() - 1 {
      None
    } else {
      Some((
        T::from_indexed_data(self.index, self.data, self.current as usize),
        T::create_index_indicator(self.index, self.current as usize),
      ))
    }
  }
}

impl<'a, V: Positioned3D, T: PrimitiveData<V>> ExactSizeIterator
  for IndexedPrimitiveIter<'a, V, T>
{
  fn len(&self) -> usize {
    self.index.len() / T::DATA_STRIDE - self.current
  }
}

/////

pub struct IndexedPrimitiveIterForPrimitiveOnly<'a, V: Positioned3D, T: PrimitiveData<V>>(
  pub IndexedPrimitiveIter<'a, V, T>,
);

impl<'a, V: Positioned3D, T: PrimitiveData<V>> Iterator
  for IndexedPrimitiveIterForPrimitiveOnly<'a, V, T>
{
  type Item = T;

  fn next(&mut self) -> Option<T> {
    self.0.next().map(|r| r.0)
  }
}

impl<'a, V: Positioned3D, T: PrimitiveData<V>> ExactSizeIterator
  for IndexedPrimitiveIterForPrimitiveOnly<'a, V, T>
{
  fn len(&self) -> usize {
    self.0.len()
  }
}
