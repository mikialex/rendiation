use rendiation_math::VectorType;
use rendiation_math_entity::LineSegment;
use rendiation_math_entity::Point;
use rendiation_math_entity::Triangle;
use std::hash::Hash;

use super::IndexType;

pub trait HashAbleByConversion {
  type HashAble: Hash + Eq;
  fn to_hashable(&self) -> Self::HashAble;
}

pub trait PrimitiveData<V: AsRef<VectorType<f32, 3>>> {
  fn from_data(data: &[V], offset: usize) -> Self;
}

pub trait IndexedPrimitiveData<I, V: AsRef<VectorType<f32, 3>>>: PrimitiveData<V> {
  type IndexIndicator;
  fn from_indexed_data(index: &[I], data: &[V], offset: usize) -> Self;
  fn create_index_indicator(index: &[I], offset: usize) -> Self::IndexIndicator;
}

impl<V: AsRef<VectorType<f32, 3>> + Copy> PrimitiveData<V> for Triangle<V> {
  #[inline(always)]
  fn from_data(data: &[V], offset: usize) -> Self {
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

impl<I: IndexType, V: AsRef<VectorType<f32, 3>> + Copy> IndexedPrimitiveData<I, V> for Triangle<V> {
  type IndexIndicator = Triangle<I>;
  #[inline(always)]
  fn from_indexed_data(index: &[I], data: &[V], offset: usize) -> Self {
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

impl<V: AsRef<VectorType<f32, 3>> + Copy> PrimitiveData<V> for LineSegment<V> {
  #[inline(always)]
  fn from_data(data: &[V], offset: usize) -> Self {
    let start = data[offset];
    let end = data[offset + 1];
    LineSegment { start, end }
  }
}

impl<I: IndexType, V: AsRef<VectorType<f32, 3>> + Copy> IndexedPrimitiveData<I, V>
  for LineSegment<V>
{
  type IndexIndicator = LineSegment<I>;
  #[inline(always)]
  fn from_indexed_data(index: &[I], data: &[V], offset: usize) -> Self {
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

impl<V: AsRef<VectorType<f32, 3>> + Copy> PrimitiveData<V> for Point<V> {
  #[inline(always)]
  fn from_data(data: &[V], offset: usize) -> Self {
    Point(data[offset])
  }
}

impl<I: IndexType, V: AsRef<VectorType<f32, 3>> + Copy> IndexedPrimitiveData<I, V> for Point<V> {
  type IndexIndicator = I;
  #[inline(always)]
  fn from_indexed_data(index: &[I], data: &[V], offset: usize) -> Self {
    Point(data[index[offset].into_usize()])
  }

  #[inline(always)]
  fn create_index_indicator(index: &[I], offset: usize) -> Self::IndexIndicator {
    index[offset]
  }
}

pub trait PrimitiveTopology<V: AsRef<VectorType<f32, 3>>>: 'static {
  type Primitive: PrimitiveData<V>;
  const STEP: usize;
  const STRIDE: usize;
  const ENUM: rendiation_ral::PrimitiveTopology;
}

pub trait IndexPrimitiveTopology<I, V>: PrimitiveTopology<V>
where
  V: AsRef<VectorType<f32, 3>>,
  <Self as PrimitiveTopology<V>>::Primitive: IndexedPrimitiveData<I, V>,
{
}

pub struct PointList;
impl<V: AsRef<VectorType<f32, 3>> + Copy> PrimitiveTopology<V> for PointList {
  type Primitive = Point<V>;
  const STEP: usize = 1;
  const STRIDE: usize = 1;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::PointList;
}
impl<I: IndexType, V: AsRef<VectorType<f32, 3>> + Copy> IndexPrimitiveTopology<I, V> for PointList {}

pub struct TriangleList;
impl<V: AsRef<VectorType<f32, 3>> + Copy> PrimitiveTopology<V> for TriangleList {
  type Primitive = Triangle<V>;
  const STEP: usize = 3;
  const STRIDE: usize = 3;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::TriangleList;
}
impl<I: IndexType, V: AsRef<VectorType<f32, 3>> + Copy> IndexPrimitiveTopology<I, V>
  for TriangleList
{
}

pub struct TriangleStrip;
impl<V: AsRef<VectorType<f32, 3>> + Copy> PrimitiveTopology<V> for TriangleStrip {
  type Primitive = Triangle<V>;
  const STEP: usize = 1;
  const STRIDE: usize = 3;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::TriangleStrip;
}
impl<I: IndexType, V: AsRef<VectorType<f32, 3>> + Copy> IndexPrimitiveTopology<I, V>
  for TriangleStrip
{
}

pub struct LineList;
impl<V: AsRef<VectorType<f32, 3>> + Copy> PrimitiveTopology<V> for LineList {
  type Primitive = LineSegment<V>;
  const STEP: usize = 2;
  const STRIDE: usize = 2;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::LineList;
}
impl<I: IndexType, V: AsRef<VectorType<f32, 3>> + Copy> IndexPrimitiveTopology<I, V> for LineList {}

pub struct LineStrip;
impl<V: AsRef<VectorType<f32, 3>> + Copy> PrimitiveTopology<V> for LineStrip {
  type Primitive = LineSegment<V>;
  const STEP: usize = 1;
  const STRIDE: usize = 2;
  const ENUM: rendiation_ral::PrimitiveTopology = rendiation_ral::PrimitiveTopology::LineStrip;
}
impl<I: IndexType, V: AsRef<VectorType<f32, 3>> + Copy> IndexPrimitiveTopology<I, V> for LineStrip {}
