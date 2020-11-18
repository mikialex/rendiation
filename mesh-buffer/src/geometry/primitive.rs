use rendiation_math_entity::LineSegment;
use rendiation_math_entity::Triangle;
use rendiation_math_entity::{Point, Positioned3D};
use std::hash::Hash;

use super::IndexType;

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
  const ENUM: rendiation_ral::PrimitiveTopology;
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
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::PointList;
}
impl<I: IndexType, T: Positioned3D> IndexPrimitiveTopology<I, T> for PointList {}

pub struct TriangleList;
impl<T: Positioned3D> PrimitiveTopology<T> for TriangleList {
  type Primitive = Triangle<T>;
  const STEP: usize = 3;
  const STRIDE: usize = 3;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::TriangleList;
}
impl<I: IndexType, T: Positioned3D> IndexPrimitiveTopology<I, T> for TriangleList {}

pub struct TriangleStrip;
impl<T: Positioned3D> PrimitiveTopology<T> for TriangleStrip {
  type Primitive = Triangle<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 3;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::TriangleStrip;
}
impl<I: IndexType, T: Positioned3D> IndexPrimitiveTopology<I, T> for TriangleStrip {}

pub struct LineList;
impl<T: Positioned3D> PrimitiveTopology<T> for LineList {
  type Primitive = LineSegment<T>;
  const STEP: usize = 2;
  const STRIDE: usize = 2;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::LineList;
}
impl<I: IndexType, T: Positioned3D> IndexPrimitiveTopology<I, T> for LineList {}

pub struct LineStrip;
impl<T: Positioned3D> PrimitiveTopology<T> for LineStrip {
  type Primitive = LineSegment<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 2;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::LineStrip;
}
impl<I: IndexType, T: Positioned3D> IndexPrimitiveTopology<I, T> for LineStrip {}
