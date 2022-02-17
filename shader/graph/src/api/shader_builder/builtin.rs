use crate::*;

macro_rules! only_vertex {
  ($Type: ident, $NodeType: ty) => {
    pub struct $Type;
    impl SemanticVertexShaderValue for $Type {
      type ValueType = $NodeType;
    }
  };
}

only_vertex!(GeometryPosition, Vec3<f32>);
only_vertex!(GeometryNormal, Vec3<f32>);
only_vertex!(GeometryUV, Vec2<f32>);
only_vertex!(GeometryColor, Vec3<f32>);
only_vertex!(GeometryColorWithAlpha, Vec4<f32>);

only_vertex!(WorldVertexPosition, Vec3<f32>);
only_vertex!(ClipPosition, Vec4<f32>);

macro_rules! both {
  ($Type: ident, $NodeType: ty) => {
    pub struct $Type;
    impl SemanticVertexShaderValue for $Type {
      type ValueType = $NodeType;
    }
    impl SemanticFragmentShaderValue for $Type {
      type ValueType = $NodeType;
    }
  };
}

both!(FragmentUv, Vec2<f32>);
both!(FragmentColor, Vec2<f32>);
both!(FragmentColorAndAlpha, Vec2<f32>);
