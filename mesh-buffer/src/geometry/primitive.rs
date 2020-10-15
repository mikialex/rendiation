use rendiation_math_entity::LineSegment;
use rendiation_math_entity::Triangle;
use rendiation_math_entity::{Point, Positioned3D};
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

impl<T: Positioned3D> PrimitiveData<T> for Point<T> {
  type IndexIndicator = u16;
  const DATA_STRIDE: usize = 1;
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

pub trait PrimitiveTopology<T: Positioned3D>: 'static {
  type Primitive: PrimitiveData<T>;
  const STRIDE: usize;
}

pub struct PointList;
impl<T: Positioned3D> PrimitiveTopology<T> for PointList {
  type Primitive = Point<T>;
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
