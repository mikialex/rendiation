use rendiation_geometry::LineSegment;
use rendiation_geometry::Point;
use rendiation_geometry::Triangle;
use std::hash::Hash;

pub trait HashAbleByConversion {
  type HashAble: Hash + Eq;
  fn to_hashable(&self) -> Self::HashAble;
}

pub trait PrimitiveData<T, U: AsRef<[T]>> {
  fn from_data(data: &U, offset: usize) -> Self;
}

pub trait IndexedPrimitiveData<I, T, U, IU>: PrimitiveData<T, U>
where
  T: Copy,
  U: AsRef<[T]>,
  IU: AsRef<[I]>,
{
  type IndexIndicator;
  fn from_indexed_data(index: &IU, data: &U, offset: usize) -> Self;
  fn create_index_indicator(index: &IU, offset: usize) -> Self::IndexIndicator;
}

impl<T, U> PrimitiveData<T, U> for Triangle<T>
where
  T: Copy,
  U: AsRef<[T]>,
{
  #[inline(always)]
  fn from_data(data: &U, offset: usize) -> Self {
    let data = data.as_ref();
    let a = data[offset];
    let b = data[offset + 1];
    let c = data[offset + 2];
    Triangle { a, b, c }
  }
}

impl<I, T, U, IU> IndexedPrimitiveData<I, T, U, IU> for Triangle<T>
where
  I: IndexType,
  T: Copy,
  U: AsRef<[T]>,
  IU: AsRef<[I]>,
{
  type IndexIndicator = Triangle<I>;
  #[inline(always)]
  fn from_indexed_data(index: &IU, data: &U, offset: usize) -> Self {
    let data = data.as_ref();
    let index = index.as_ref();
    let a = data[index[offset].into()];
    let b = data[index[offset + 1].into()];
    let c = data[index[offset + 2].into()];
    Triangle { a, b, c }
  }

  #[inline(always)]
  fn create_index_indicator(index: &IU, offset: usize) -> Self::IndexIndicator {
    let index = index.as_ref();
    let a = index[offset];
    let b = index[offset + 1];
    let c = index[offset + 2];
    Triangle { a, b, c }
  }
}

impl<T, U> PrimitiveData<T, U> for LineSegment<T>
where
  T: Copy,
  U: AsRef<[T]>,
{
  #[inline(always)]
  fn from_data(data: &U, offset: usize) -> Self {
    let data = data.as_ref();
    let start = data[offset];
    let end = data[offset + 1];
    LineSegment { start, end }
  }
}

impl<I, T, U, IU> IndexedPrimitiveData<I, T, U, IU> for LineSegment<T>
where
  I: IndexType,
  T: Copy,
  U: AsRef<[T]>,
  IU: AsRef<[I]>,
{
  type IndexIndicator = LineSegment<I>;
  #[inline(always)]
  fn from_indexed_data(index: &IU, data: &U, offset: usize) -> Self {
    let index = index.as_ref();
    let data = data.as_ref();
    let start = data[index[offset].into()];
    let end = data[index[offset + 1].into()];
    LineSegment { start, end }
  }
  #[inline(always)]
  fn create_index_indicator(index: &IU, offset: usize) -> Self::IndexIndicator {
    let index = index.as_ref();
    let start = index[offset];
    let end = index[offset + 1];
    LineSegment { start, end }
  }
}

impl<T, U> PrimitiveData<T, U> for Point<T>
where
  T: Copy,
  U: AsRef<[T]>,
{
  #[inline(always)]
  fn from_data(data: &U, offset: usize) -> Self {
    let data = data.as_ref();
    Point(data[offset])
  }
}

impl<I, T, U, IU> IndexedPrimitiveData<I, T, U, IU> for Point<T>
where
  I: IndexType,
  T: Copy,
  U: AsRef<[T]>,
  IU: AsRef<[I]>,
{
  type IndexIndicator = I;
  #[inline(always)]
  fn from_indexed_data(index: &IU, data: &U, offset: usize) -> Self {
    let index = index.as_ref();
    let data = data.as_ref();
    Point(data[index[offset].into()])
  }

  #[inline(always)]
  fn create_index_indicator(index: &IU, offset: usize) -> Self::IndexIndicator {
    let index = index.as_ref();
    index[offset]
  }
}

pub trait PrimitiveTopologyMeta<T>: 'static {
  type Primitive;
  const STEP: usize;
  const STRIDE: usize;
  const ENUM: PrimitiveTopology;
}

pub trait IndexPrimitiveTopologyMeta<I, T>: PrimitiveTopologyMeta<T> {}

/// Primitive type the input mesh is composed of.
#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum PrimitiveTopology {
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

pub struct PointList;
impl<T> PrimitiveTopologyMeta<T> for PointList {
  type Primitive = Point<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 1;
  const ENUM: PrimitiveTopology = PrimitiveTopology::PointList;
}
impl<I: IndexType, T> IndexPrimitiveTopologyMeta<I, T> for PointList {}

pub struct TriangleList;
impl<T> PrimitiveTopologyMeta<T> for TriangleList {
  type Primitive = Triangle<T>;
  const STEP: usize = 3;
  const STRIDE: usize = 3;
  const ENUM: PrimitiveTopology = PrimitiveTopology::TriangleList;
}
impl<I: IndexType, T> IndexPrimitiveTopologyMeta<I, T> for TriangleList {}

pub struct TriangleStrip;
impl<T> PrimitiveTopologyMeta<T> for TriangleStrip {
  type Primitive = Triangle<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 3;
  const ENUM: PrimitiveTopology = PrimitiveTopology::TriangleStrip;
}
impl<I: IndexType, T> IndexPrimitiveTopologyMeta<I, T> for TriangleStrip {}

pub struct LineList;
impl<T> PrimitiveTopologyMeta<T> for LineList {
  type Primitive = LineSegment<T>;
  const STEP: usize = 2;
  const STRIDE: usize = 2;
  const ENUM: PrimitiveTopology = PrimitiveTopology::LineList;
}
impl<I: IndexType, T> IndexPrimitiveTopologyMeta<I, T> for LineList {}

pub struct LineStrip;
impl<T> PrimitiveTopologyMeta<T> for LineStrip {
  type Primitive = LineSegment<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 2;
  const ENUM: PrimitiveTopology = PrimitiveTopology::LineStrip;
}
impl<I: IndexType, T> IndexPrimitiveTopologyMeta<I, T> for LineStrip {}

use super::IndexType;
