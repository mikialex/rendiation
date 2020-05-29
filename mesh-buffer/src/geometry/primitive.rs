use super::intersection::MeshBufferIntersectionConfigProvider;
use core::marker::PhantomData;
use rendiation_math_entity::Face3;
use rendiation_math_entity::IntersectAble;
use rendiation_math_entity::Line3;
use rendiation_math_entity::NearestPoint3D;
use rendiation_math_entity::{Point, PositionedPoint, Ray};
use std::hash::Hash;

pub trait HashAbleByConversion {
  type HashAble: Hash + Eq;
  fn to_hashable(&self) -> Self::HashAble;
}

pub trait PrimitiveData<T: PositionedPoint> {
  type IndexIndicator;
  fn from_indexed_data(index: &[u16], data: &[T], offset: usize) -> Self;
  fn create_index_indicator(index: &[u16], offset: usize) -> Self::IndexIndicator;
  fn from_data(data: &[T], offset: usize) -> Self;
}

impl<T: PositionedPoint> PrimitiveData<T> for Face3<T> {
  type IndexIndicator = Face3<u16>;
  fn from_indexed_data(index: &[u16], data: &[T], offset: usize) -> Self {
    let a = data[index[offset] as usize];
    let b = data[index[offset + 1] as usize];
    let c = data[index[offset + 2] as usize];
    Face3 { a, b, c }
  }

  fn create_index_indicator(index: &[u16], offset: usize) -> Self::IndexIndicator {
    let a = index[offset];
    let b = index[offset + 1];
    let c = index[offset + 2];
    Face3 { a, b, c }
  }

  fn from_data(data: &[T], offset: usize) -> Self {
    let a = data[offset];
    let b = data[offset + 1];
    let c = data[offset + 2];
    Face3 { a, b, c }
  }
}

impl<T: PositionedPoint> PrimitiveData<T> for Line3<T> {
  type IndexIndicator = Line3<u16>;
  fn from_indexed_data(index: &[u16], data: &[T], offset: usize) -> Self {
    let start = data[index[offset] as usize];
    let end = data[index[offset + 1] as usize];
    Line3 { start, end }
  }
  fn create_index_indicator(index: &[u16], offset: usize) -> Self::IndexIndicator {
    let start = index[offset];
    let end = index[offset + 1];
    Line3 { start, end }
  }
  fn from_data(data: &[T], offset: usize) -> Self {
    let start = data[offset];
    let end = data[offset + 1];
    Line3 { start, end }
  }
}

impl<T: PositionedPoint> PrimitiveData<T> for Point<T> {
  type IndexIndicator = u16;
  fn from_indexed_data(index: &[u16], data: &[T], offset: usize) -> Self {
    Point(data[index[offset] as usize])
  }

  fn create_index_indicator(index: &[u16], offset: usize) -> Self::IndexIndicator {
    index[offset]
  }

  fn from_data(data: &[T], offset: usize) -> Self {
    Point(data[offset])
  }
}

pub trait PrimitiveTopology<T: PositionedPoint> {
  type Primitive: PrimitiveData<T>
    + IntersectAble<Ray, Option<NearestPoint3D>, Box<dyn MeshBufferIntersectionConfigProvider>>;
  const STRIDE: usize;
}

pub struct PointList;
impl<T: PositionedPoint> PrimitiveTopology<T> for PointList {
  type Primitive = Point<T>;
  const STRIDE: usize = 1;
}

pub struct TriangleList;
impl<T: PositionedPoint> PrimitiveTopology<T> for TriangleList {
  type Primitive = Face3<T>;
  const STRIDE: usize = 3;
}

pub struct TriangleStrip;
impl<T: PositionedPoint> PrimitiveTopology<T> for TriangleStrip {
  type Primitive = Face3<T>;
  const STRIDE: usize = 1;
}

pub struct LineList;
impl<T: PositionedPoint> PrimitiveTopology<T> for LineList {
  type Primitive = Line3<T>;
  const STRIDE: usize = 2;
}

pub struct LineStrip;
impl<T: PositionedPoint> PrimitiveTopology<T> for LineStrip {
  type Primitive = Line3<T>;
  const STRIDE: usize = 1;
}

pub struct IndexedPrimitiveIter<'a, V: PositionedPoint, T: PrimitiveData<V>> {
  index: &'a [u16],
  data: &'a [V],
  current: i16,
  _phantom: PhantomData<T>,
}

impl<'a, V: PositionedPoint, T: PrimitiveData<V>> Iterator for IndexedPrimitiveIter<'a, V, T> {
  type Item = (T, T::IndexIndicator);

  fn next(&mut self) -> Option<(T, T::IndexIndicator)> {
    self.current += 1;
    if self.current == self.index.len() as i16 {
      None
    } else {
      Some((
        T::from_indexed_data(self.index, self.data, self.current as usize),
        T::create_index_indicator(self.index, self.current as usize),
      ))
    }
  }
}

impl<'a, V: PositionedPoint, T: PrimitiveData<V>> IndexedPrimitiveIter<'a, V, T> {
  pub fn new(index: &'a [u16], data: &'a [V]) -> Self {
    Self {
      index,
      data,
      current: -1,
      _phantom: PhantomData,
    }
  }
}

pub struct PrimitiveIter<'a, V: PositionedPoint, T: PrimitiveData<V>> {
  data: &'a [V],
  current: i16,
  _phantom: PhantomData<T>,
}

impl<'a, V: PositionedPoint, T: PrimitiveData<V>> PrimitiveIter<'a, V, T> {
  pub fn new(data: &'a [V]) -> Self {
    Self {
      data,
      current: -1,
      _phantom: PhantomData,
    }
  }
}

impl<'a, V: PositionedPoint, T: PrimitiveData<V>> Iterator for PrimitiveIter<'a, V, T> {
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
