use crate::*;

pub struct GLSL {
  pub version: usize,
}

impl ShaderGraphCodeGenTarget for GLSL {
  fn gen_primitive_type(&self, ty: crate::PrimitiveShaderValueType) -> &'static str {
    match ty {
      crate::PrimitiveShaderValueType::Float32 => "float",
      crate::PrimitiveShaderValueType::Vec2Float32 => "vec2",
      crate::PrimitiveShaderValueType::Vec3Float32 => "vec3",
      crate::PrimitiveShaderValueType::Vec4Float32 => "vec4",
      crate::PrimitiveShaderValueType::Mat2Float32 => "mat2",
      crate::PrimitiveShaderValueType::Mat3Float32 => "mat3",
      crate::PrimitiveShaderValueType::Mat4Float32 => "mat4",
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
        "texture(sampler2D({}, {}), {})",
        builder.get_node_gen_result_var(n.texture),
        builder.get_node_gen_result_var(n.sampler),
        builder.get_node_gen_result_var(n.position),
      ),
      ShaderGraphNodeData::Swizzle { ty, source } => todo!(),
      ShaderGraphNodeData::Compose(_) => todo!(),
      ShaderGraphNodeData::Operator(_) => todo!(),
      ShaderGraphNodeData::Input(_) => return None,
      ShaderGraphNodeData::Named(_) => return None,
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
