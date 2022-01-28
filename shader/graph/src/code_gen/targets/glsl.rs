use crate::*;

pub struct GLSL {
  pub version: usize,
}

impl ShaderGraphCodeGenTarget for GLSL {
  fn gen_primitive_literal(&self, v: PrimitiveShaderValue) -> String {
    gen_primitive_literal_common(self, v)
  }
  fn gen_primitive_type(&self, ty: PrimitiveShaderValueType) -> &'static str {
    match ty {
      PrimitiveShaderValueType::Bool => "bool",
      PrimitiveShaderValueType::Float32 => "float",
      PrimitiveShaderValueType::Vec2Float32 => "vec2",
      PrimitiveShaderValueType::Vec3Float32 => "vec3",
      PrimitiveShaderValueType::Vec4Float32 => "vec4",
      PrimitiveShaderValueType::Mat2Float32 => "mat2",
      PrimitiveShaderValueType::Mat3Float32 => "mat3",
      PrimitiveShaderValueType::Mat4Float32 => "mat4",
      PrimitiveShaderValueType::Uint32 => "int",
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
      ShaderGraphNodeData::Swizzle { ty, source } => {
        format!("{}.{}", builder.get_node_gen_result_var(*source), ty)
      }
      ShaderGraphNodeData::Compose { .. } => todo!(),
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
      ShaderGraphNodeData::Copy(node) => {
        format!("{}", builder.get_node_gen_result_var(*node))
      }
    };
    expr.into()
  }
}
