use crate::{
  geometry::{AnyGeometry, IndexType, IndexedGeometry, NoneIndexedGeometry, TriangleList},
  range::GeometryRangesInfo,
  vertex::Vertex,
};

pub mod cube;
pub mod cylinder;
pub mod plane;
pub mod sphere;
pub use cube::*;
pub use cylinder::*;
pub use plane::*;
pub use sphere::*;

// todo add support for index overflow check
pub trait IndexedGeometryTessellator<T = Vertex, I: IndexType = u16, P = TriangleList> {
  fn tessellate(&self) -> TesselationResult<IndexedGeometry<I, T, P>>;
}

pub trait NoneIndexedGeometryTessellator<T = Vertex, P = TriangleList> {
  fn tessellate(&self) -> TesselationResult<NoneIndexedGeometry<T, P>>;
}

pub struct TesselationResult<T> {
  pub geometry: T,
  pub range: GeometryRangesInfo,
}

impl<T: AnyGeometry> TesselationResult<T> {
  pub fn new(geometry: T, range: GeometryRangesInfo) -> Self {
    Self { geometry, range }
  }
  pub fn full_range(geometry: T) -> Self {
    let range = GeometryRangesInfo::full_range(&geometry);
    Self { geometry, range }
  }
}
