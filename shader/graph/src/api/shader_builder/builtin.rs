use crate::*;

pub struct GeometryLocalSpacePosition;
pub struct GeometryLocalSpaceNormal;
pub struct GeometryUV;

impl SemanticVertexShaderValue for GeometryLocalSpacePosition {
  type ValueType = Vec3<f32>;
}

impl SemanticVertexShaderValue for GeometryLocalSpaceNormal {
  type ValueType = Vec3<f32>;
}

impl SemanticVertexShaderValue for GeometryUV {
  type ValueType = Vec2<f32>;
}
