use crate::*;

pub struct WGSL;

impl ShaderGraphCodeGenTarget for WGSL {
  fn gen_primitive_literal(&self, v: PrimitiveShaderValue) -> String {
    gen_primitive_literal_common(self, v)
  }

  fn gen_primitive_type(&self, ty: PrimitiveShaderValueType) -> &'static str {
    match ty {
      PrimitiveShaderValueType::Float32 => "f32",
      PrimitiveShaderValueType::Vec2Float32 => "vec2<f32>",
      PrimitiveShaderValueType::Vec3Float32 => "vec3<f32>",
      PrimitiveShaderValueType::Vec4Float32 => "vec4<f32>",
      PrimitiveShaderValueType::Mat2Float32 => "mat2x2<f32>",
      PrimitiveShaderValueType::Mat3Float32 => "mat3x3<f32>",
      PrimitiveShaderValueType::Mat4Float32 => "mat4x4<f32>",
      PrimitiveShaderValueType::Uint32 => "u32",
      PrimitiveShaderValueType::Bool => "bool",
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
      ShaderGraphNodeData::Swizzle { ty, source } => {
        format!("{}.{}", builder.get_node_gen_result_var(*source), ty)
      }
      ShaderGraphNodeData::Operator(o) => {
        let left = builder.get_node_gen_result_var(o.left);
        let right = builder.get_node_gen_result_var(o.right);
        format!("{} {} {}", left, o.operator, right)
      }
      ShaderGraphNodeData::Input(_) => return None,
      ShaderGraphNodeData::Named(name) => name.clone(),
      ShaderGraphNodeData::FieldGet {
        // todo should this merged with swizzle
        field_name,
        struct_node,
      } => format!(
        "{}.{}",
        builder.get_node_gen_result_var(*struct_node),
        field_name
      ),
      ShaderGraphNodeData::StructConstruct { struct_id, fields } => todo!(),
      ShaderGraphNodeData::Const(ConstNode { data }) => self.gen_primitive_literal(*data),
      ShaderGraphNodeData::Copy(node) => builder.get_node_gen_result_var(*node).to_owned(),
      ShaderGraphNodeData::Scope(_) => todo!(),
      ShaderGraphNodeData::Compose { target, parameters } => {
        format!(
          "{}({})",
          self.gen_primitive_type(*target),
          parameters
            .iter()
            .map(|from| { builder.get_node_gen_result_var(*from) })
            .collect::<Vec<_>>()
            .join(", ")
        )
      }
    };
    expr.into()
  }

  fn gen_vertex_shader(
    &self,
    vertex: &mut ShaderGraphVertexBuilder,
    builder: &mut ShaderGraphBuilder,
  ) -> String {
    todo!()
  }

  fn gen_fragment_shader(
    &self,
    vertex: &mut ShaderGraphFragmentBuilder,
    builder: &mut ShaderGraphBuilder,
  ) -> String {
    todo!()
  }
}

fn gen_entry(stage: ShaderStages, content: String) -> String {
  let name = match stage {
    ShaderStages::Vertex => "vertex",
    ShaderStages::Fragment => "fragment",
  };

  format!(
    "
[[stage({name})]]
fn {name}_main(input) -> {{
{content}
}}
"
  )
}
