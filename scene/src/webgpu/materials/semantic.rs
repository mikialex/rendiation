use rendiation_algebra::*;
use shadergraph::SemanticVertexFragmentIOValue;

pub struct FragmentUv;

impl SemanticVertexFragmentIOValue for FragmentUv {
  type ValueType = Vec2<f32>;
}

pub struct FragmentColor;

impl SemanticVertexFragmentIOValue for FragmentColor {
  type ValueType = Vec3<f32>;
}

pub struct FragmentColorAndAlpha;

impl SemanticVertexFragmentIOValue for FragmentColorAndAlpha {
  type ValueType = Vec4<f32>;
}
