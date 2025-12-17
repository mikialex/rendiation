use crate::*;

#[derive(Clone)]
pub struct MeshBufferIntersectConfig {
  /// applicable only if is line or point
  ///
  /// triangle can implement it but it's too costly
  pub tolerance_local: f32,
  /// applicable only if is triangle
  pub triangle_face: FaceSide,
}

impl IntersectAble<AttributeDynPrimitive, OptionalNearest<HitPoint3D>, MeshBufferIntersectConfig>
  for Ray3
{
  fn intersect(
    &self,
    pri: &AttributeDynPrimitive,
    param: &MeshBufferIntersectConfig,
  ) -> OptionalNearest<HitPoint3D> {
    match pri {
      AttributeDynPrimitive::Points(v) => self.intersect(v, &param.tolerance_local),
      AttributeDynPrimitive::LineSegment(v) => self.intersect(v, &param.tolerance_local),
      AttributeDynPrimitive::Triangle(v) => self.intersect(v, &param.triangle_face),
    }
  }
}

impl SpaceBounding<f32, Box3, 3> for AttributeDynPrimitive {
  fn to_bounding(&self) -> Box3 {
    match self {
      AttributeDynPrimitive::Points(v) => v.to_bounding(),
      AttributeDynPrimitive::LineSegment(v) => v.to_bounding(),
      AttributeDynPrimitive::Triangle(v) => v.to_bounding(),
    }
  }
}
