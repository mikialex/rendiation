use rendiation_math_entity::LineSegment;
use rendiation_math_entity::Triangle;
use rendiation_math_entity::{Point, Positioned};
use std::{hash::Hash, ops::Index};

pub trait HashAbleByConversion {
  type HashAble: Hash + Eq;
  fn to_hashable(&self) -> Self::HashAble;
}

pub trait PrimitiveData<T: Positioned<f32, 3>, U: Index<usize, Output = T>> {
  fn from_data(data: &U, offset: usize) -> Self;
}

pub trait IndexedPrimitiveData<I, T, U, IU>: PrimitiveData<T, U>
where
  T: Positioned<f32, 3>,
  U: Index<usize, Output = T>,
  IU: Index<usize, Output = I>,
{
  type IndexIndicator;
  fn from_indexed_data(index: &IU, data: &U, offset: usize) -> Self;
  fn create_index_indicator(index: &IU, offset: usize) -> Self::IndexIndicator;
}

impl<T, U> PrimitiveData<T, U> for Triangle<T>
where
  T: Positioned<f32, 3>,
  U: Index<usize, Output = T>,
{
  #[inline(always)]
  fn from_data(data: &U, offset: usize) -> Self {
    let a = data[offset];
    let b = data[offset + 1];
    let c = data[offset + 2];
    Triangle { a, b, c }
  }
}

impl<I, T, U, IU> IndexedPrimitiveData<I, T, U, IU> for Triangle<T>
where
  I: IndexType,
  T: Positioned<f32, 3>,
  U: Index<usize, Output = T>,
  IU: Index<usize, Output = I>,
{
  type IndexIndicator = Triangle<I>;
  #[inline(always)]
  fn from_indexed_data(index: &IU, data: &U, offset: usize) -> Self {
    let a = data[index[offset].into_usize()];
    let b = data[index[offset + 1].into_usize()];
    let c = data[index[offset + 2].into_usize()];
    Triangle { a, b, c }
  }

  #[inline(always)]
  fn create_index_indicator(index: &IU, offset: usize) -> Self::IndexIndicator {
    let a = index[offset];
    let b = index[offset + 1];
    let c = index[offset + 2];
    Triangle { a, b, c }
  }
}

impl<T, U> PrimitiveData<T, U> for LineSegment<T>
where
  T: Positioned<f32, 3>,
  U: Index<usize, Output = T>,
{
  #[inline(always)]
  fn from_data(data: &U, offset: usize) -> Self {
    let start = data[offset];
    let end = data[offset + 1];
    LineSegment { start, end }
  }
}

impl<I, T, U, IU> IndexedPrimitiveData<I, T, U, IU> for LineSegment<T>
where
  I: IndexType,
  T: Positioned<f32, 3>,
  U: Index<usize, Output = T>,
  IU: Index<usize, Output = I>,
{
  type IndexIndicator = LineSegment<I>;
  #[inline(always)]
  fn from_indexed_data(index: &IU, data: &U, offset: usize) -> Self {
    let start = data[index[offset].into_usize()];
    let end = data[index[offset + 1].into_usize()];
    LineSegment { start, end }
  }
  #[inline(always)]
  fn create_index_indicator(index: &IU, offset: usize) -> Self::IndexIndicator {
    let start = index[offset];
    let end = index[offset + 1];
    LineSegment { start, end }
  }
}

impl<T, U> PrimitiveData<T, U> for Point<T>
where
  T: Positioned<f32, 3>,
  U: Index<usize, Output = T>,
{
  #[inline(always)]
  fn from_data(data: &U, offset: usize) -> Self {
    Point(data[offset])
  }
}

impl<I, T, U, IU> IndexedPrimitiveData<I, T, U, IU> for Point<T>
where
  I: IndexType,
  T: Positioned<f32, 3>,
  U: Index<usize, Output = T>,
  IU: Index<usize, Output = I>,
{
  type IndexIndicator = I;
  #[inline(always)]
  fn from_indexed_data(index: &IU, data: &U, offset: usize) -> Self {
    Point(data[index[offset].into_usize()])
  }

  #[inline(always)]
  fn create_index_indicator(index: &IU, offset: usize) -> Self::IndexIndicator {
    index[offset]
  }
}

pub trait PrimitiveTopology<T: Positioned<f32, 3>>: 'static {
  type Primitive;
  const STEP: usize;
  const STRIDE: usize;
  const ENUM: rendiation_ral::PrimitiveTopology;
}

pub trait IndexPrimitiveTopology<I, T>: PrimitiveTopology<T>
where
  T: Positioned<f32, 3>,
{
}

pub struct PointList;
impl<T: Positioned<f32, 3>> PrimitiveTopology<T> for PointList {
  type Primitive = Point<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 1;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::PointList;
}
impl<I: IndexType, T: Positioned<f32, 3>> IndexPrimitiveTopology<I, T> for PointList {}

pub struct TriangleList;
impl<T: Positioned<f32, 3>> PrimitiveTopology<T> for TriangleList {
  type Primitive = Triangle<T>;
  const STEP: usize = 3;
  const STRIDE: usize = 3;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::TriangleList;
}
impl<I: IndexType, T: Positioned<f32, 3>> IndexPrimitiveTopology<I, T> for TriangleList {}

pub struct TriangleStrip;
impl<T: Positioned<f32, 3>> PrimitiveTopology<T> for TriangleStrip {
  type Primitive = Triangle<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 3;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::TriangleStrip;
}
impl<I: IndexType, T: Positioned<f32, 3>> IndexPrimitiveTopology<I, T> for TriangleStrip {}

pub struct LineList;
impl<T: Positioned<f32, 3>> PrimitiveTopology<T> for LineList {
  type Primitive = LineSegment<T>;
  const STEP: usize = 2;
  const STRIDE: usize = 2;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::LineList;
}
impl<I: IndexType, T: Positioned<f32, 3>> IndexPrimitiveTopology<I, T> for LineList {}

pub struct LineStrip;
impl<T: Positioned<f32, 3>> PrimitiveTopology<T> for LineStrip {
  type Primitive = LineSegment<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 2;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::LineStrip;
}
impl<I: IndexType, T: Positioned<f32, 3>> IndexPrimitiveTopology<I, T> for LineStrip {}

use super::IndexType;
