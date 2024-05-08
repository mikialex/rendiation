use crate::*;

// we should consider merge it with other similar trait
pub trait Simplex: IntoIterator<Item = Self::Vertex> {
  type Vertex;
  type Topology;
  const TOPOLOGY: PrimitiveTopology;
  const DIMENSION: usize;
}

impl<V> Simplex for Point<V> {
  type Vertex = V;
  type Topology = PointList;
  const TOPOLOGY: PrimitiveTopology = PrimitiveTopology::PointList;
  const DIMENSION: usize = 1;
}
impl<V> Simplex for LineSegment<V> {
  type Vertex = V;
  type Topology = LineList;
  const TOPOLOGY: PrimitiveTopology = PrimitiveTopology::LineList;
  const DIMENSION: usize = 2;
}
impl<V> Simplex for Triangle<V> {
  type Vertex = V;
  type Topology = TriangleList;
  const TOPOLOGY: PrimitiveTopology = PrimitiveTopology::TriangleList;
  const DIMENSION: usize = 3;
}

pub trait PrimitiveData<U>: Sized {
  fn from_data(data: &U, offset: usize) -> Option<Self>;
  /// ## Safety
  ///
  /// Users should responsible for offset is in bound, bound checking is skipped here
  unsafe fn from_data_unchecked(data: &U, offset: usize) -> Self;
}

impl<T, U> PrimitiveData<U> for Triangle<T>
where
  T: Copy,
  U: IndexGet<Output = T>,
{
  #[inline(always)]
  fn from_data(data: &U, offset: usize) -> Option<Self> {
    let a = data.index_get(offset)?;
    let b = data.index_get(offset + 1)?;
    let c = data.index_get(offset + 2)?;
    Triangle { a, b, c }.into()
  }
  #[inline(always)]
  unsafe fn from_data_unchecked(data: &U, offset: usize) -> Self {
    let a = data.index_get(offset).unwrap_unchecked();
    let b = data.index_get(offset + 1).unwrap_unchecked();
    let c = data.index_get(offset + 2).unwrap_unchecked();
    Triangle { a, b, c }
  }
}

impl<T, U> PrimitiveData<U> for LineSegment<T>
where
  T: Copy,
  U: IndexGet<Output = T>,
{
  #[inline(always)]
  fn from_data(data: &U, offset: usize) -> Option<Self> {
    let start = data.index_get(offset)?;
    let end = data.index_get(offset + 1)?;
    LineSegment::new(start, end).into()
  }
  #[inline(always)]
  unsafe fn from_data_unchecked(data: &U, offset: usize) -> Self {
    let start = data.index_get(offset).unwrap_unchecked();
    let end = data.index_get(offset + 1).unwrap_unchecked();
    LineSegment::new(start, end)
  }
}

impl<T, U> PrimitiveData<U> for Point<T>
where
  T: Copy,
  U: IndexGet<Output = T>,
{
  #[inline(always)]
  fn from_data(data: &U, offset: usize) -> Option<Self> {
    Point(data.index_get(offset)?).into()
  }
  #[inline(always)]
  unsafe fn from_data_unchecked(data: &U, offset: usize) -> Self {
    Point(data.index_get(offset).unwrap_unchecked())
  }
}

pub type FunctorInner<T> = <T as Functor>::Unwrapped;
pub type FunctorMapped<T, U> = <T as Functor>::Wrapped<U>;
/// we should move this trait to math/geometry?
pub trait Functor {
  type Unwrapped;
  type Wrapped<B>: Functor;

  fn f_map<F, B>(self, f: F) -> Self::Wrapped<B>
  where
    F: FnMut(Self::Unwrapped) -> B;

  fn f_filter_map<F, B>(self, f: F) -> Option<Self::Wrapped<B>>
  where
    F: FnMut(Self::Unwrapped) -> Option<B>;
}

impl<A> Functor for Triangle<A> {
  type Unwrapped = A;
  type Wrapped<B> = Triangle<B>;

  fn f_map<F: FnMut(A) -> B, B>(self, f: F) -> Triangle<B> {
    self.map(f)
  }

  fn f_filter_map<F: FnMut(A) -> Option<B>, B>(self, f: F) -> Option<Triangle<B>> {
    self.filter_map(f)
  }
}

impl<A> Functor for LineSegment<A> {
  type Unwrapped = A;
  type Wrapped<B> = LineSegment<B>;

  fn f_map<F: FnMut(A) -> B, B>(self, f: F) -> LineSegment<B> {
    self.map(f)
  }

  fn f_filter_map<F: FnMut(A) -> Option<B>, B>(self, f: F) -> Option<LineSegment<B>> {
    self.filter_map(f)
  }
}

impl<A> Functor for Point<A> {
  type Unwrapped = A;
  type Wrapped<B> = Point<B>;

  fn f_map<F: FnMut(A) -> B, B>(self, f: F) -> Point<B> {
    self.map(f)
  }

  fn f_filter_map<F: FnMut(A) -> Option<B>, B>(self, f: F) -> Option<Point<B>> {
    self.filter_map(f)
  }
}

pub trait PrimitiveTopologyMeta: 'static {
  type Primitive<V>: Functor;
  const STEP: usize;
  const STRIDE: usize;
  const ENUM: PrimitiveTopology;
}

/// Primitive type the input mesh is composed of.
#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Default)]
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
  #[default]
  TriangleList = 3,
  /// Vertex data is a triangle strip. Each set of three adjacent vertices form a triangle.
  ///
  /// Vertices `0 1 2 3 4 5` creates four triangles `0 1 2`, `2 1 3`, `2 3 4`, and `4 3 5`
  TriangleStrip = 4,
}

impl PrimitiveTopology {
  pub fn stride(&self) -> usize {
    match self {
      PrimitiveTopology::PointList => 1,
      PrimitiveTopology::LineList => 2,
      PrimitiveTopology::LineStrip => 2,
      PrimitiveTopology::TriangleList => 3,
      PrimitiveTopology::TriangleStrip => 3,
    }
  }

  pub fn step(&self) -> usize {
    match self {
      PrimitiveTopology::PointList => 1,
      PrimitiveTopology::LineList => 2,
      PrimitiveTopology::LineStrip => 1,
      PrimitiveTopology::TriangleList => 3,
      PrimitiveTopology::TriangleStrip => 1,
    }
  }
}

#[derive(Clone, Default)]
pub struct PointList;
impl PrimitiveTopologyMeta for PointList {
  type Primitive<T> = Point<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 1;
  const ENUM: PrimitiveTopology = PrimitiveTopology::PointList;
}

#[derive(Clone, Default)]
pub struct TriangleList;
impl PrimitiveTopologyMeta for TriangleList {
  type Primitive<T> = Triangle<T>;
  const STEP: usize = 3;
  const STRIDE: usize = 3;
  const ENUM: PrimitiveTopology = PrimitiveTopology::TriangleList;
}

#[derive(Clone, Default)]
pub struct TriangleStrip;
impl PrimitiveTopologyMeta for TriangleStrip {
  type Primitive<T> = Triangle<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 3;
  const ENUM: PrimitiveTopology = PrimitiveTopology::TriangleStrip;
}

#[derive(Clone, Default)]
pub struct LineList;
impl PrimitiveTopologyMeta for LineList {
  type Primitive<T> = LineSegment<T>;
  const STEP: usize = 2;
  const STRIDE: usize = 2;
  const ENUM: PrimitiveTopology = PrimitiveTopology::LineList;
}

#[derive(Clone, Default)]
pub struct LineStrip;
impl PrimitiveTopologyMeta for LineStrip {
  type Primitive<T> = LineSegment<T>;
  const STEP: usize = 1;
  const STRIDE: usize = 2;
  const ENUM: PrimitiveTopology = PrimitiveTopology::LineStrip;
}
