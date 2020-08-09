use core::marker::PhantomData;
use rendiation_math_entity::LineSegment;
use rendiation_math_entity::Triangle;
use rendiation_math_entity::{Point3, Positioned3D};
use std::hash::Hash;

pub trait HashAbleByConversion {
  type HashAble: Hash + Eq;
  fn to_hashable(&self) -> Self::HashAble;
}

pub trait PrimitiveData<T: Positioned3D> {
  type IndexIndicator;
  const DATA_STRIDE: usize;
  fn from_indexed_data(index: &[u16], data: &[T], offset: usize) -> Self;
  fn create_index_indicator(index: &[u16], offset: usize) -> Self::IndexIndicator;
  fn from_data(data: &[T], offset: usize) -> Self;
}

impl<T: Positioned3D> PrimitiveData<T> for Triangle<T> {
  type IndexIndicator = Triangle<u16>;
  const DATA_STRIDE: usize = 3;
  fn from_indexed_data(index: &[u16], data: &[T], offset: usize) -> Self {
    let a = data[index[offset] as usize];
    let b = data[index[offset + 1] as usize];
    let c = data[index[offset + 2] as usize];
    Triangle { a, b, c }
  }

  fn create_index_indicator(index: &[u16], offset: usize) -> Self::IndexIndicator {
    let a = index[offset];
    let b = index[offset + 1];
    let c = index[offset + 2];
    Triangle { a, b, c }
  }

  fn from_data(data: &[T], offset: usize) -> Self {
    let a = data[offset];
    let b = data[offset + 1];
    let c = data[offset + 2];
    Triangle { a, b, c }
  }
}

impl<T: Positioned3D> PrimitiveData<T> for LineSegment<T> {
  type IndexIndicator = LineSegment<u16>;
  const DATA_STRIDE: usize = 2;
  fn from_indexed_data(index: &[u16], data: &[T], offset: usize) -> Self {
    let start = data[index[offset] as usize];
    let end = data[index[offset + 1] as usize];
    LineSegment { start, end }
  }
  fn create_index_indicator(index: &[u16], offset: usize) -> Self::IndexIndicator {
    let start = index[offset];
    let end = index[offset + 1];
    LineSegment { start, end }
  }
  fn from_data(data: &[T], offset: usize) -> Self {
    let start = data[offset];
    let end = data[offset + 1];
    LineSegment { start, end }
  }
}

impl<T: Positioned3D> PrimitiveData<T> for Point3<T> {
  type IndexIndicator = u16;
  const DATA_STRIDE: usize = 1;
  fn from_indexed_data(index: &[u16], data: &[T], offset: usize) -> Self {
    Point3(data[index[offset] as usize])
  }

  fn create_index_indicator(index: &[u16], offset: usize) -> Self::IndexIndicator {
    index[offset]
  }

  fn from_data(data: &[T], offset: usize) -> Self {
    Point3(data[offset])
  }
}

pub trait PrimitiveTopology<T: Positioned3D> {
  type Primitive: PrimitiveData<T>;
  const STRIDE: usize;
}

pub struct PointList;
impl<T: Positioned3D> PrimitiveTopology<T> for PointList {
  type Primitive = Point3<T>;
  const STRIDE: usize = 1;
}

pub struct TriangleList;
impl<T: Positioned3D> PrimitiveTopology<T> for TriangleList {
  type Primitive = Triangle<T>;
  const STRIDE: usize = 3;
}

pub struct TriangleStrip;
impl<T: Positioned3D> PrimitiveTopology<T> for TriangleStrip {
  type Primitive = Triangle<T>;
  const STRIDE: usize = 1;
}

pub struct LineList;
impl<T: Positioned3D> PrimitiveTopology<T> for LineList {
  type Primitive = LineSegment<T>;
  const STRIDE: usize = 2;
}

pub struct LineStrip;
impl<T: Positioned3D> PrimitiveTopology<T> for LineStrip {
  type Primitive = LineSegment<T>;
  const STRIDE: usize = 1;
}

pub struct IndexedPrimitiveIter<'a, V: Positioned3D, T: PrimitiveData<V>> {
  index: &'a [u16],
  data: &'a [V],
  current: usize,
  _phantom: PhantomData<T>,
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
  // We can easily calculate the remaining number of iterations.
  fn len(&self) -> usize {
    self.index.len() / T::DATA_STRIDE - self.current
  }
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

pub struct PrimitiveIter<'a, V: Positioned3D, T: PrimitiveData<V>> {
  data: &'a [V],
  current: i16,
  _phantom: PhantomData<T>,
}

impl<'a, V: Positioned3D, T: PrimitiveData<V>> PrimitiveIter<'a, V, T> {
  pub fn new(data: &'a [V]) -> Self {
    Self {
      data,
      current: -1,
      _phantom: PhantomData,
    }
  }
}

impl<'a, V: Positioned3D, T: PrimitiveData<V>> Iterator for PrimitiveIter<'a, V, T> {
  type Item = T;

  fn next(&mut self) -> Option<T> {
    self.current += 1;
    if self.current == self.data.len() as i16 {
      None
    } else {
      Some(T::from_data(self.data, self.current as usize))
    }
  }
}
