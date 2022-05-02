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

only_vertex!(CameraProjectionMatrix, Mat4<f32>);
only_vertex!(CameraViewMatrix, Mat4<f32>);

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
both!(FragmentAlpha, f32);
both!(FragmentSpecular, Vec3<f32>);
both!(FragmentSpecularShininess, f32);
both!(FragmentColor, Vec3<f32>);
both!(FragmentColorAndAlpha, Vec4<f32>);

both!(RenderBufferSize, Vec2<f32>);
