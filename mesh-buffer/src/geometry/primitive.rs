use crate::vertex::Vertex;
use core::marker::PhantomData;
use rendiation_math::Vec3;
use rendiation_math_entity::Face3;
use rendiation_math_entity::IntersectAble;
use rendiation_math_entity::Line3;
use rendiation_math_entity::NearestPoint3D;
use rendiation_math_entity::Ray;

pub trait PositionedPoint {
  fn position(&self) -> Vec3<f32>;
}

impl PositionedPoint for Vertex {
  fn position(&self) -> Vec3<f32> {
    self.position
  }
}

pub trait PrimitiveFromGeometryData<T: PositionedPoint> {
  fn from_indexed_data(index: &[u16], data: &[T], offset: usize) -> Self;
  fn from_data(data: &[T], offset: usize) -> Self;
}

impl<T: PositionedPoint> PrimitiveFromGeometryData<T> for Face3 {
  fn from_indexed_data(index: &[u16], data: &[T], offset: usize) -> Self {
    let a = data[index[offset] as usize].position();
    let b = data[index[offset + 1] as usize].position();
    let c = data[index[offset + 2] as usize].position();
    Face3 { a, b, c }
  }

  fn from_data(data: &[T], offset: usize) -> Self {
    let a = data[offset].position();
    let b = data[offset + 1].position();
    let c = data[offset + 2].position();
    Face3 { a, b, c }
  }
}

impl<T: PositionedPoint> PrimitiveFromGeometryData<T> for Line3 {
  fn from_indexed_data(index: &[u16], data: &[T], offset: usize) -> Self {
    let start = data[index[offset] as usize].position();
    let end = data[index[offset + 1] as usize].position();
    Line3 { start, end }
  }
  fn from_data(data: &[T], offset: usize) -> Self {
    let start = data[offset].position();
    let end = data[offset + 1].position();
    Line3 { start, end }
  }
}

pub trait PrimitiveTopology<T: PositionedPoint> {
  type Primitive: PrimitiveFromGeometryData<T> + IntersectAble<Ray, Option<NearestPoint3D>>;
  const STRIDE: usize;
}

pub struct TriangleList;

impl<T: PositionedPoint> PrimitiveTopology<T> for TriangleList {
  type Primitive = Face3;
  const STRIDE: usize = 3;
}

pub struct LineList;

impl<T: PositionedPoint> PrimitiveTopology<T> for LineList {
  type Primitive = Line3;
  const STRIDE: usize = 2;
}

pub struct IndexedPrimitiveIter<'a, V: PositionedPoint, T: PrimitiveFromGeometryData<V>> {
  index: &'a [u16],
  data: &'a [V],
  current: i16,
  _phantom: PhantomData<T>,
}

impl<'a, V: PositionedPoint, T: PrimitiveFromGeometryData<V>> Iterator
  for IndexedPrimitiveIter<'a, V, T>
{
  type Item = T;

  fn next(&mut self) -> Option<T> {
    self.current += 1;
    if self.current == self.index.len() as i16 {
      None
    } else {
      Some(T::from_indexed_data(
        self.index,
        self.data,
        self.current as usize,
      ))
    }
  }
}

impl<'a, V: PositionedPoint, T: PrimitiveFromGeometryData<V>> IndexedPrimitiveIter<'a, V, T> {
  pub fn new(index: &'a [u16], data: &'a [V]) -> Self {
    Self {
      index,
      data,
      current: -1,
      _phantom: PhantomData,
    }
  }
}

pub struct PrimitiveIter<'a, V: PositionedPoint, T: PrimitiveFromGeometryData<V>> {
  data: &'a [V],
  current: i16,
  _phantom: PhantomData<T>,
}

impl<'a, V: PositionedPoint, T: PrimitiveFromGeometryData<V>> PrimitiveIter<'a, V, T> {
  pub fn new(data: &'a [V]) -> Self {
    Self {
      data,
      current: -1,
      _phantom: PhantomData,
    }
  }
}

impl<'a, V: PositionedPoint, T: PrimitiveFromGeometryData<V>> Iterator for PrimitiveIter<'a, V, T> {
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
