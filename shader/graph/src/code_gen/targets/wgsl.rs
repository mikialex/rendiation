use crate::*;

pub struct WGSL;

impl ShaderGraphCodeGenTarget for WGSL {
  fn gen_primitive_type(&self, ty: crate::PrimitiveShaderValueType) -> &'static str {
    match ty {
      crate::PrimitiveShaderValueType::Float32 => "f32",
      crate::PrimitiveShaderValueType::Vec2Float32 => "vec2<f32>",
      crate::PrimitiveShaderValueType::Vec3Float32 => "vec3<f32>",
      crate::PrimitiveShaderValueType::Vec4Float32 => "vec4<f32>",
      crate::PrimitiveShaderValueType::Mat2Float32 => "mat2x2<f32>",
      crate::PrimitiveShaderValueType::Mat3Float32 => "mat3x3<f32>",
      crate::PrimitiveShaderValueType::Mat4Float32 => "mat4x4<f32>",
    }
  }

  fn gen_expr(
    &self,
    data: &ShaderGraphNodeData,
    builder: &mut ShaderGraphBuilder,
  ) -> Option<String> {
    let expr = match data {
      ShaderGraphNodeData::Function(n) => {
        builder.add_fn_dep(n);
        format!(
          "{}({})",
          n.prototype.function_name,
          n.parameters
            .iter()
            .map(|from| { builder.get_node_gen_result_var(*from) })
            .collect::<Vec<_>>()
            .join(", ")
        )
      }
      ShaderGraphNodeData::BuiltInFunction { name, parameters } => todo!(),
      ShaderGraphNodeData::TextureSampling(n) => format!(
        "textureSample({}, {}, {})",
        builder.get_node_gen_result_var(n.texture),
        builder.get_node_gen_result_var(n.sampler),
        builder.get_node_gen_result_var(n.position),
      ),
      ShaderGraphNodeData::Swizzle { ty, source } => todo!(),
      ShaderGraphNodeData::Compose(_) => todo!(),
      ShaderGraphNodeData::Operator(_) => todo!(),
      ShaderGraphNodeData::Input(_) => todo!(),
      ShaderGraphNodeData::Named(_) => todo!(),
      ShaderGraphNodeData::FieldGet {
        field_name,
        struct_node,
      } => todo!(),
      ShaderGraphNodeData::StructConstruct { struct_id, fields } => todo!(),
      ShaderGraphNodeData::Const(_) => todo!(),
      ShaderGraphNodeData::Scope(_) => todo!(),
    };
    expr.into()
  }
}
