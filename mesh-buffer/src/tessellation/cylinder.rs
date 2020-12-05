use crate::{
  geometry::{IndexedGeometry, TriangleList},
  vertex::Vertex,
};

use super::{IndexedGeometryTessellator, TesselationResult};

#[derive(Copy, Clone, Debug)]
pub struct PlaneGeometryParameter {
  pub radiusTop: f32,
  pub radiusBottom: f32,
  pub height: f32,
  pub radialSegments: usize,
  pub heightSegments: usize,
  pub openEnded: bool,
  pub thetaStart: f32,
  pub thetaLength: f32,
}

impl IndexedGeometryTessellator for PlaneGeometryParameter {
  fn tessellate(&self) -> TesselationResult<IndexedGeometry<u16, Vertex, TriangleList>> {
    todo!()
  }
}
