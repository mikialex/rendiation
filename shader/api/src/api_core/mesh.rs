use crate::*;

pub enum MeshOutputTopology {
  /// Outputs individual vertices to be rendered as points.
  Points,
  /// Outputs groups of 2 vertices to be rendered as lines .
  Lines,
  /// Outputs groups of 3 vertices to be rendered as triangles.
  Triangles,
}

impl MeshOutputTopology {
  pub fn as_struct_field_name(&self) -> &'static str {
    match self {
      MeshOutputTopology::Points => "points",
      MeshOutputTopology::Lines => "lines",
      MeshOutputTopology::Triangles => "triangles",
    }
  }
  pub fn data_type(&self) -> PrimitiveShaderValueType {
    match self {
      MeshOutputTopology::Points => PrimitiveShaderValueType::u32(),
      MeshOutputTopology::Lines => PrimitiveShaderValueType::vec2::<u32>(),
      MeshOutputTopology::Triangles => PrimitiveShaderValueType::vec3::<u32>(),
    }
  }
  pub fn deco(&self) -> ShaderBuiltInDecorator {
    match self {
      MeshOutputTopology::Points => ShaderBuiltInDecorator::MeshPrimitivePointIndex,
      MeshOutputTopology::Lines => ShaderBuiltInDecorator::MeshPrimitiveLineIndex,
      MeshOutputTopology::Triangles => ShaderBuiltInDecorator::MeshPrimitiveTriangleIndex,
    }
  }
}

pub struct MeshStageInfo {
  /// The type of primitive outputted.
  pub topology: MeshOutputTopology,
  /// The maximum number of vertices a mesh shader may output.
  pub max_vertices: u32,
  /// The maximum number of primitives a mesh shader may output.
  pub max_primitives: u32,
  /// The type used by vertex outputs, i.e. what is passed to `setVertex`.
  pub vertex_output_type: ShaderSizedValueType,
  /// The type used by primitive outputs, i.e. what is passed to `setPrimitive`.
  pub primitive_output_type: ShaderSizedValueType,
  /// The global variable holding the outputted vertices, primitives, and counts
  pub output_variable: ShaderNodeRawHandle,
}
