use rendiation_math_entity::LineSegment;
use rendiation_math_entity::Triangle;
use rendiation_math_entity::{Point, Positioned3D};
use std::hash::Hash;

pub trait HashAbleByConversion {
  type HashAble: Hash + Eq;
  fn to_hashable(&self) -> Self::HashAble;
}

pub trait PrimitiveData<T: Positioned3D> {
  fn from_data(data: &[T], offset: usize) -> Self;
}

pub trait IndexedPrimitiveData<I, T: Positioned3D>: PrimitiveData<T> {
  type IndexIndicator;
  fn from_indexed_data(index: &[I], data: &[T], offset: usize) -> Self;
  fn create_index_indicator(index: &[I], offset: usize) -> Self::IndexIndicator;
}

impl<T: Positioned3D> PrimitiveData<T> for Triangle<T> {
  #[inline(always)]
  fn from_data(data: &[T], offset: usize) -> Self {
    unsafe {
      let a = *data.get_unchecked(offset);
      let b = *data.get_unchecked(offset + 1);
      let c = *data.get_unchecked(offset + 2);
      Triangle { a, b, c }
    }
    // let a = data[offset];
    // let b = data[offset + 1];
    // let c = data[offset + 2];
    // Triangle { a, b, c }
  }
}

impl<I: IndexType, T: Positioned3D> IndexedPrimitiveData<I, T> for Triangle<T> {
  type IndexIndicator = Triangle<I>;
  #[inline(always)]
  fn from_indexed_data(index: &[I], data: &[T], offset: usize) -> Self {
    unsafe {
      let a = *data.get_unchecked(index.get_unchecked(offset).into_usize());
      let b = *data.get_unchecked(index.get_unchecked(offset + 1).into_usize());
      let c = *data.get_unchecked(index.get_unchecked(offset + 2).into_usize());
      Triangle { a, b, c }
    }
    // let a = data[index[offset].into_usize()];
    // let b = data[index[offset + 1].into_usize()];
    // let c = data[index[offset + 2].into_usize()];
    // Triangle { a, b, c }
  }

  #[inline(always)]
  fn create_index_indicator(index: &[I], offset: usize) -> Self::IndexIndicator {
    let a = index[offset];
    let b = index[offset + 1];
    let c = index[offset + 2];
    Triangle { a, b, c }
  }
}

impl<T: Positioned3D> PrimitiveData<T> for LineSegment<T> {
  #[inline(always)]
  fn from_data(data: &[T], offset: usize) -> Self {
    let start = data[offset];
    let end = data[offset + 1];
    LineSegment { start, end }
  }
}

impl<I: IndexType, T: Positioned3D> IndexedPrimitiveData<I, T> for LineSegment<T> {
  type IndexIndicator = LineSegment<I>;
  #[inline(always)]
  fn from_indexed_data(index: &[I], data: &[T], offset: usize) -> Self {
    let start = data[index[offset].into_usize()];
    let end = data[index[offset + 1].into_usize()];
    LineSegment { start, end }
  }
  #[inline(always)]
  fn create_index_indicator(index: &[I], offset: usize) -> Self::IndexIndicator {
    let start = index[offset];
    let end = index[offset + 1];
    LineSegment { start, end }
  }
}

impl<T: Positioned3D> PrimitiveData<T> for Point<T> {
  #[inline(always)]
  fn from_data(data: &[T], offset: usize) -> Self {
    Point(data[offset])
  }
}

impl<I: IndexType, T: Positioned3D> IndexedPrimitiveData<I, T> for Point<T> {
  type IndexIndicator = I;
  #[inline(always)]
  fn from_indexed_data(index: &[I], data: &[T], offset: usize) -> Self {
    Point(data[index[offset].into_usize()])
  }

  #[inline(always)]
  fn create_index_indicator(index: &[I], offset: usize) -> Self::IndexIndicator {
    index[offset]
  }
}

pub trait PrimitiveTopology<T: Positioned3D>: 'static {
  type Primitive: PrimitiveData<T>;
  const STEP: usize;
  const STRIDE: usize;
  const ENUM: PrimitiveTopologyEnum;
}

pub trait IndexPrimitiveTopology<I, T>: PrimitiveTopology<T>
where
  T: Positioned3D,
  <Self as PrimitiveTopology<T>>::Primitive: IndexedPrimitiveData<I, T>,
{
}

pub struct PointList;
impl<T: Positioned3D> PrimitiveTopology<T> for PointList {
  type Primitive = Point<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 1;
  const ENUM: PrimitiveTopologyEnum = PrimitiveTopologyEnum::PointList;
}
impl<I: IndexType, T: Positioned3D> IndexPrimitiveTopology<I, T> for PointList {}

pub struct TriangleList;
impl<T: Positioned3D> PrimitiveTopology<T> for TriangleList {
  type Primitive = Triangle<T>;
  const STEP: usize = 3;
  const STRIDE: usize = 3;
  const ENUM: PrimitiveTopologyEnum = PrimitiveTopologyEnum::TriangleList;
}
impl<I: IndexType, T: Positioned3D> IndexPrimitiveTopology<I, T> for TriangleList {}

pub struct TriangleStrip;
impl<T: Positioned3D> PrimitiveTopology<T> for TriangleStrip {
  type Primitive = Triangle<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 3;
  const ENUM: PrimitiveTopologyEnum = PrimitiveTopologyEnum::TriangleStrip;
}
impl<I: IndexType, T: Positioned3D> IndexPrimitiveTopology<I, T> for TriangleStrip {}

pub struct LineList;
impl<T: Positioned3D> PrimitiveTopology<T> for LineList {
  type Primitive = LineSegment<T>;
  const STEP: usize = 2;
  const STRIDE: usize = 2;
  const ENUM: PrimitiveTopologyEnum = PrimitiveTopologyEnum::LineList;
}
impl<I: IndexType, T: Positioned3D> IndexPrimitiveTopology<I, T> for LineList {}

pub struct LineStrip;
impl<T: Positioned3D> PrimitiveTopology<T> for LineStrip {
  type Primitive = LineSegment<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 2;
  const ENUM: PrimitiveTopologyEnum = PrimitiveTopologyEnum::LineStrip;
}
impl<I: IndexType, T: Positioned3D> IndexPrimitiveTopology<I, T> for LineStrip {}

use wasm_bindgen::prelude::*;

use super::IndexType;

#[wasm_bindgen]
#[derive(Copy, Clone, Debug)]
pub enum PrimitiveTopologyEnum {
  /// Vertex data is a list of points. Each vertex is a new point.
  PointList = 0,
  /// Vertex data is a list of lines. Each pair of vertices composes a new line.
  ///
  /// Vertices `0 1 2 3` create two lines `0 1` and `2 3`
  LineList = 1,
  /// Vertex data is a strip of lines. Each set of two adjacent vertices form a line.
  ///
  /// Vertices `0 1 2 3` create three lines `0 1`, `1 2`, and `2 3`.
  LineStrip = 2,
  /// Vertex data is a list of triangles. Each set of 3 vertices composes a new triangle.
  ///
  /// Vertices `0 1 2 3 4 5` create two triangles `0 1 2` and `3 4 5`
  TriangleList = 3,
  /// Vertex data is a triangle strip. Each set of three adjacent vertices form a triangle.
  ///
  /// Vertices `0 1 2 3 4 5` creates four triangles `0 1 2`, `2 1 3`, `3 2 4`, and `4 3 5`
  TriangleStrip = 4,
}
