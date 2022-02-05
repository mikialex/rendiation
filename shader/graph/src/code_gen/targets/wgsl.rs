use crate::*;

pub struct WGSL;

impl ShaderGraphCodeGenTarget for WGSL {
  fn gen_primitive_literal(&self, v: PrimitiveShaderValue) -> String {
    gen_primitive_literal_common(self, v)
  }

  fn gen_primitive_type(&self, ty: PrimitiveShaderValueType) -> &'static str {
    gen_primitive_type_impl(ty)
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
      ShaderGraphNodeData::Scope => return None,
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

  fn gen_statement(
    &self,
    expr: &ShaderGraphNodeData,
    builder: &mut ShaderGraphBuilder,
  ) -> Option<(String, String)> {
    let name = builder.top_scope().code_gen.create_new_unique_name();
    let expr = self.gen_expr(expr, builder)?;
    let statement = format!("let {name} = {expr};");
    (name, statement).into()
  }

  fn gen_vertex_shader(
    &self,
    vertex: &mut ShaderGraphVertexBuilder,
    builder: ShaderGraphBuilder,
  ) -> String {
    let mut code = CodeBuilder::default();
    gen_structs(&mut code, &builder);
    gen_bindings(&mut code, &vertex.bindgroups, ShaderStages::Vertex);
    builder.gen_fn_depends(&mut code);
    gen_entry(&mut code, ShaderStages::Vertex, builder.compile());
    code.output()
  }

  fn gen_fragment_shader(
    &self,
    vertex: &mut ShaderGraphFragmentBuilder,
    builder: ShaderGraphBuilder,
  ) -> String {
    let mut code = CodeBuilder::default();
    gen_structs(&mut code, &builder);
    gen_bindings(&mut code, &vertex.bindgroups, ShaderStages::Fragment);
    builder.gen_fn_depends(&mut code);
    gen_entry(&mut code, ShaderStages::Fragment, builder.compile());
    code.output()
  }
}

fn gen_structs(code: &mut CodeBuilder, builder: &ShaderGraphBuilder) {
  builder
    .struct_defines
    .iter()
    .for_each(|(_, meta)| gen_struct(code, meta))
}

fn gen_struct(builder: &mut CodeBuilder, meta: &ShaderStructMetaInfo) {
  builder.write_ln(format!("struct {} {{", meta.name));
  builder.tab();
  for (field_name, ty) in &meta.fields {
    builder.write_ln(format!("{}: {};", field_name, gen_fix_type_impl(*ty)));
  }
  builder.un_tab();
  builder.write_ln("}}");
}

fn gen_bindings(
  code: &mut CodeBuilder,
  builder: &ShaderGraphBindGroupBuilder,
  stage: ShaderStages,
) {
  builder
    .bindings
    .iter()
    .enumerate()
    .for_each(|(group_index, b)| {
      b.bindings
        .iter()
        .enumerate()
        .for_each(|(item_index, (entry, _))| {
          gen_bind_entry(code, entry, group_index, item_index, stage)
        });
    })
}

fn gen_bind_entry(
  code: &mut CodeBuilder,
  entry: &ShaderGraphBindEntry,
  group_index: usize,
  item_index: usize,
  stage: ShaderStages,
) {
  if match stage {
    ShaderStages::Vertex => entry.used_in_vertex,
    ShaderStages::Fragment => entry.used_in_fragment,
  } {
    code.write_ln(format!(
      "[[group({}), binding({})]] var{} {}: {};",
      group_index,
      item_index,
      match entry.ty {
        ShaderValueType::Fixed(_) => "<uniform>",
        _ => "",
      },
      "unnamed_todo",
      gen_type_impl(entry.ty),
    ));
  }
}

fn gen_entry(code: &mut CodeBuilder, stage: ShaderStages, content: String) {
  let name = match stage {
    ShaderStages::Vertex => "vertex",
    ShaderStages::Fragment => "fragment",
  };

  code.write_ln(format!("[[stage({name})]]"));
  code.write_ln(format!("fn {name}_main(input) -> {{"));
  code.write_raw(content);
  code.write_ln("}");
}

fn gen_primitive_type_impl(ty: PrimitiveShaderValueType) -> &'static str {
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

fn gen_type_impl(ty: ShaderValueType) -> String {
  match ty {
    ShaderValueType::Sampler => "sampler".to_owned(),
    ShaderValueType::Texture => "texture_2d<f32>".to_owned(),
    ShaderValueType::Fixed(ty) => gen_fix_type_impl(ty).to_owned(),
    ShaderValueType::Never => unreachable!("can not code generate never type"),
  }
}

fn gen_fix_type_impl(ty: ShaderStructMemberValueType) -> &'static str {
  match ty {
    ShaderStructMemberValueType::Primitive(ty) => gen_primitive_type_impl(ty),
    ShaderStructMemberValueType::Struct(meta) => meta.name,
  }
}
