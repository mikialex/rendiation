use crate::*;

#[macro_export]
macro_rules! only_vertex {
  ($Type: ident, $NodeType: ty) => {
    pub struct $Type;
    impl SemanticVertexShaderValue for $Type {
      type ValueType = $NodeType;
    }
  };
}

#[macro_export]
macro_rules! only_fragment {
  ($Type: ident, $NodeType: ty) => {
    pub struct $Type;
    impl SemanticFragmentShaderValue for $Type {
      type ValueType = $NodeType;
    }
  };
}

#[macro_export]
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

//////
// wgsl builtin https://www.w3.org/TR/WGSL/#builtin-values

// vertex input
only_vertex!(VertexIndex, u32);
only_vertex!(VertexInstanceIndex, u32);

// vertex output
only_vertex!(ClipPosition, Vec4<f32>);

// fragment input
both!(FragmentFrontFacing, bool);
both!(FragmentPosition, Vec4<f32>);
both!(FragmentSampleIndex, u32);
both!(FragmentSampleMaskInput, u32);

// fragment output
both!(FragmentDepthOutput, f32);
both!(FragmentSampleMaskOutput, u32);

//////
// shader graph builtin

only_vertex!(GeometryPosition2D, Vec2<f32>);
only_vertex!(GeometryPosition, Vec3<f32>);
only_vertex!(GeometryNormal, Vec3<f32>);

pub type GeometryUV = GeometryUVChannel<0>;
pub struct GeometryUVChannel<const I: usize>;
impl<const I: usize> SemanticVertexShaderValue for GeometryUVChannel<I> {
  type ValueType = Vec2<f32>;
}

pub struct JointIndexChannel<const I: usize>;
impl<const I: usize> SemanticVertexShaderValue for JointIndexChannel<I> {
  type ValueType = u32; // todo support u8 u16
}

pub struct WeightChannel<const I: usize>;
impl<const I: usize> SemanticVertexShaderValue for WeightChannel<I> {
  type ValueType = f32;
}

only_vertex!(GeometryColor, Vec3<f32>);
only_vertex!(GeometryColorWithAlpha, Vec4<f32>);

both!(WorldMatrix, Mat4<f32>);
only_vertex!(WorldVertexPosition, Vec3<f32>);
only_vertex!(WorldVertexNormal, Vec3<f32>);

both!(CameraProjectionMatrix, Mat4<f32>);
both!(CameraProjectionInverseMatrix, Mat4<f32>);
both!(CameraViewMatrix, Mat4<f32>);
both!(CameraWorldMatrix, Mat4<f32>);
both!(CameraViewProjectionMatrix, Mat4<f32>);
both!(CameraViewProjectionInverseMatrix, Mat4<f32>);

only_fragment!(DefaultDisplay, Vec4<f32>);

both!(FragmentUv, Vec2<f32>);
both!(FragmentWorldPosition, Vec3<f32>);
both!(FragmentWorldNormal, Vec3<f32>);
both!(FragmentColor, Vec3<f32>);
both!(FragmentColorAndAlpha, Vec4<f32>); // todo remove

both!(RenderBufferSize, Vec2<f32>);
both!(TexelSize, Vec2<f32>);
