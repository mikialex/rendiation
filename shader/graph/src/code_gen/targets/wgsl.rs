use crate::*;

pub struct WGSL;

impl ShaderGraphCodeGenTarget for WGSL {
  fn gen_vertex_shader(
    &self,
    vertex: &mut ShaderGraphVertexBuilder,
    builder: ShaderGraphBuilder,
  ) -> String {
    let mut code = CodeBuilder::default();
    let mut cx = CodeGenCtx::default();

    gen_structs(&mut code, &builder);
    gen_bindings(&mut code, &vertex.bindgroups, ShaderStages::Vertex);
    gen_entry(&mut code, ShaderStages::Vertex, |code| {
      gen_node_with_dep_in_entry(
        vertex.vertex_position.handle().cast_untyped(),
        &builder,
        &mut cx,
        code,
      );

      gen_node_with_dep_in_entry(
        vertex.vertex_point_size.handle().cast_untyped(),
        &builder,
        &mut cx,
        code,
      );

      vertex.vertex_out.iter().for_each(|(_, (v, _))| {
        gen_node_with_dep_in_entry(v.handle(), &builder, &mut cx, code);
      })
    });
    cx.gen_fn_depends(&mut code);
    code.output()
  }

  fn gen_fragment_shader(
    &self,
    fragment: &mut ShaderGraphFragmentBuilder,
    builder: ShaderGraphBuilder,
  ) -> String {
    let mut code = CodeBuilder::default();
    let mut cx = CodeGenCtx::default();
    gen_structs(&mut code, &builder);
    gen_bindings(&mut code, &fragment.bindgroups, ShaderStages::Fragment);
    gen_entry(&mut code, ShaderStages::Fragment, |code| {
      fragment.frag_output.iter().for_each(|v| {
        gen_node_with_dep_in_entry(v.handle().cast_untyped(), &builder, &mut cx, code);
      })
    });
    cx.gen_fn_depends(&mut code);
    code.output()
  }
}

fn gen_node_with_dep_in_entry(
  node: ShaderGraphNodeRawHandleUntyped,
  builder: &ShaderGraphBuilder,
  cx: &mut CodeGenCtx,
  code: &mut CodeBuilder,
) {
  let root = builder.scopes.first().unwrap();
  root.nodes.traverse_dfs_in_topological_order(
    node.handle,
    &mut |n| {
      gen_node(
        &n.data().data,
        ShaderGraphNodeRawHandle {
          handle: n.handle(),
          graph_id: node.graph_id,
        },
        cx,
        code,
      );
    },
    &mut || panic!("loop"),
  );
}

fn gen_scope_full(scope: &ShaderGraphScope, cx: &mut CodeGenCtx, code: &mut CodeBuilder) {
  let nodes = &scope.nodes;
  scope
    .inserted
    .iter()
    .for_each(|n| gen_node(&nodes.get_node(n.handle).data().data, *n, cx, code))
}

fn gen_node(
  data: &ShaderGraphNodeData,
  handle: ShaderGraphNodeRawHandleUntyped,
  cx: &mut CodeGenCtx,
  code: &mut CodeBuilder,
) {
  match data {
    ShaderGraphNodeData::Write { source, target } => code.write_ln(format!(
      "{} = {};",
      cx.get_node_gen_result_var(*target),
      cx.get_node_gen_result_var(*source)
    )),
    ShaderGraphNodeData::ControlFlow(cf) => match cf {
      ShaderControlFlowNode::If { condition, scope } => {
        code
          .write_ln(format!(
            "if ({}) {{",
            cx.get_node_gen_result_var(*condition)
          ))
          .tab();

        gen_scope_full(scope, cx, code);

        code.write_ln("}").un_tab()
      }
      ShaderControlFlowNode::For { source, scope } => {
        let name = cx.create_new_unique_name();
        let head = match source {
          ShaderIteratorAble::Const(v) => format!("for(int {name} = 0; {name} < {v}; {name}++) {{"),
          ShaderIteratorAble::Count(v) => format!(
            "for(int {name} = 0; {name} < {count}; {name}++) {{",
            count = cx.get_node_gen_result_var(v.handle().cast_untyped())
          ),
        };
        code.write_ln(head).tab();

        gen_scope_full(scope, cx, code);

        code.write_ln("}").un_tab()
      }
    },
    ShaderGraphNodeData::SideEffect(effect) => match effect {
      ShaderSideEffectNode::Continue => code.write_ln("continue;"),
      ShaderSideEffectNode::Break => code.write_ln("break;"),
      ShaderSideEffectNode::Return(v) => {
        code.write_ln(format!("return {};", cx.get_node_gen_result_var(*v)))
      }
      ShaderSideEffectNode::Termination => code.write_ln("discard;"),
    },
    expr => {
      let name = cx.create_new_unique_name();
      let expr = gen_expr(expr, cx);
      let statement = format!("let {name} = {expr};");
      code.write_ln(&statement);
      cx.top_scope_mut().code_gen_history.insert(
        handle,
        MiddleVariableCodeGenResult {
          var_name: name,
          statement,
        },
      );
      code
    }
  };
}

fn gen_expr(data: &ShaderGraphNodeData, cx: &mut CodeGenCtx) -> String {
  match data {
    ShaderGraphNodeData::FunctionCall(n) => {
      cx.add_fn_dep(n);
      format!(
        "{}({})",
        n.prototype.function_name,
        n.parameters
          .iter()
          .map(|from| { cx.get_node_gen_result_var(*from) })
          .collect::<Vec<_>>()
          .join(", ")
      )
    }
    ShaderGraphNodeData::TextureSampling(n) => format!(
      "textureSample({}, {}, {})",
      cx.get_node_gen_result_var(n.texture),
      cx.get_node_gen_result_var(n.sampler),
      cx.get_node_gen_result_var(n.position),
    ),
    ShaderGraphNodeData::Swizzle { ty, source } => {
      format!("{}.{}", cx.get_node_gen_result_var(*source), ty)
    }
    ShaderGraphNodeData::Operator(o) => {
      let left = cx.get_node_gen_result_var(o.left);
      let right = cx.get_node_gen_result_var(o.right);
      format!("{} {} {}", left, o.operator, right)
    }
    ShaderGraphNodeData::Input(input) => gen_input_name(input),
    ShaderGraphNodeData::UnNamed => todo!(),
    ShaderGraphNodeData::FieldGet {
      // todo should this merged with swizzle
      field_name,
      struct_node,
    } => format!(
      "{}.{}",
      cx.get_node_gen_result_var(*struct_node),
      field_name
    ),
    ShaderGraphNodeData::StructConstruct { struct_id, fields } => todo!(),
    ShaderGraphNodeData::Const(ConstNode { data }) => gen_primitive_literal(*data),
    ShaderGraphNodeData::Copy(node) => cx.get_node_gen_result_var(*node).to_owned(),
    ShaderGraphNodeData::Compose { target, parameters } => {
      format!(
        "{}({})",
        gen_primitive_type(*target),
        parameters
          .iter()
          .map(|from| { cx.get_node_gen_result_var(*from) })
          .collect::<Vec<_>>()
          .join(", ")
      )
    }
    ShaderGraphNodeData::Write { source, target } => todo!(),
    ShaderGraphNodeData::ControlFlow(_) => todo!(),
    ShaderGraphNodeData::SideEffect(_) => todo!(),
  }
}

fn gen_input_name(input: &ShaderGraphInputNode) -> String {
  match input {
    ShaderGraphInputNode::BuiltIn(ty) => gen_built_in(*ty).to_owned(),
    ShaderGraphInputNode::Uniform {
      bindgroup_index,
      entry_index,
    } => format!("uniform_b_{}_i_{}", bindgroup_index, entry_index),
    ShaderGraphInputNode::VertexIn { index, .. } => format!("vertex_in_{}", index),
    ShaderGraphInputNode::FragmentIn { index, .. } => format!("fragment_in_{}", index),
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

fn gen_entry(
  code: &mut CodeBuilder,
  stage: ShaderStages,
  mut content: impl FnMut(&mut CodeBuilder),
) {
  let name = match stage {
    ShaderStages::Vertex => "vertex",
    ShaderStages::Fragment => "fragment",
  };

  code.write_ln(format!("[[stage({name})]]"));
  code.write_ln(format!("fn {name}_main(input) -> {{"));
  code.tab();
  content(code);
  code.un_tab();
  code.write_ln("}");
}

fn gen_primitive_type(ty: PrimitiveShaderValueType) -> &'static str {
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
    ShaderStructMemberValueType::Primitive(ty) => gen_primitive_type(ty),
    ShaderStructMemberValueType::Struct(meta) => meta.name,
  }
}

fn gen_built_in(ty: ShaderBuiltIn) -> &'static str {
  match ty {
    ShaderBuiltIn::VertexClipPosition => "bt_vertex_clip_position",
    ShaderBuiltIn::VertexPointSize => "bt_vertex_point_size",
    ShaderBuiltIn::VertexIndexId => "bt_vertex_vertex_id",
    ShaderBuiltIn::VertexInstanceId => "bt_vertex_instance_id",
  }
}

pub fn gen_primitive_literal(v: PrimitiveShaderValue) -> String {
  let grouped = match v {
    PrimitiveShaderValue::Bool(v) => format!("{v}"),
    PrimitiveShaderValue::Float32(f) => return float_to_shader(f),
    PrimitiveShaderValue::Vec2Float32(v) => {
      let v: &[f32; 2] = v.as_ref();
      float_group(v.as_slice())
    }
    PrimitiveShaderValue::Vec3Float32(v) => {
      let v: &[f32; 3] = v.as_ref();
      float_group(v.as_slice())
    }
    PrimitiveShaderValue::Vec4Float32(v) => {
      let v: &[f32; 4] = v.as_ref();
      float_group(v.as_slice())
    }
    PrimitiveShaderValue::Mat2Float32(v) => {
      let v: &[f32; 4] = v.as_ref();
      float_group(v.as_slice())
    }
    PrimitiveShaderValue::Mat3Float32(v) => {
      let v: &[f32; 9] = v.as_ref();
      float_group(v.as_slice())
    }
    PrimitiveShaderValue::Mat4Float32(v) => {
      let v: &[f32; 16] = v.as_ref();
      float_group(v.as_slice())
    }
    PrimitiveShaderValue::Uint32(v) => format!("{}", v),
  };
  #[allow(clippy::match_like_matches_macro)]
  let require_constructor = match v {
    PrimitiveShaderValue::Bool(_) => false,
    PrimitiveShaderValue::Uint32(_) => false,
    PrimitiveShaderValue::Float32(_) => false,
    _ => true,
  };
  if require_constructor {
    format!("{}{}", gen_primitive_type(v.into()), grouped)
  } else {
    grouped
  }
}
