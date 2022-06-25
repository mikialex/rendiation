use crate::*;

pub struct GLSLes {
  pub is_es300: bool,
}

pub struct GLSLShaderSource {
  pub vertex: String,
  pub fragment: String,
  pub is_es300: bool,
}

impl ShaderGraphCodeGenTarget for GLSLes {
  type ShaderSource = GLSLShaderSource;

  fn compile(
    &self,
    builder: &ShaderGraphRenderPipelineBuilder,
    vertex: ShaderGraphBuilder,
    fragment: ShaderGraphBuilder,
  ) -> Self::ShaderSource {
    let vertex = gen_vertex_shader(builder, vertex);
    let fragment = gen_fragment_shader(builder, fragment);
    GLSLShaderSource {
      vertex,
      fragment,
      is_es300: self.is_es300,
    }
  }
}

fn gen_vertex_shader(
  pipeline_builder: &ShaderGraphRenderPipelineBuilder,
  builder: ShaderGraphBuilder,
) -> String {
  let vertex = &pipeline_builder.vertex;

  let mut code = CodeBuilder::default();
  let mut cx = CodeGenCtx::default();

  gen_structs(&mut code, &builder);
  gen_bindings(
    &mut code,
    &pipeline_builder.bindgroups,
    ShaderStages::Vertex,
  );
  gen_entry(&mut code, ShaderStages::Vertex, |code| {
    let root = gen_node_with_dep_in_entry(
      vertex.vertex_position.get().handle(),
      &builder,
      &mut cx,
      code,
    );
    code.write_ln(format!("gl_Position = {root};"));

    vertex.vertex_out.iter().for_each(|(_, (v, _, i))| {
      let root = gen_node_with_dep_in_entry(v.handle(), &builder, &mut cx, code);
      code.write_ln(format!("vertex_out{i} = {root};"));
    });
  });
  cx.gen_fn_depends(&mut code);
  code.output()
}

fn gen_fragment_shader(
  pipeline_builder: &ShaderGraphRenderPipelineBuilder,
  builder: ShaderGraphBuilder,
) -> String {
  let fragment = &pipeline_builder.fragment;

  let mut code = CodeBuilder::default();
  let mut cx = CodeGenCtx::default();
  gen_structs(&mut code, &builder);
  gen_bindings(
    &mut code,
    &pipeline_builder.bindgroups,
    ShaderStages::Fragment,
  );
  gen_entry(&mut code, ShaderStages::Fragment, |code| {
    fragment
      .frag_output
      .iter()
      .enumerate()
      .for_each(|(i, (v, _))| {
        let root = gen_node_with_dep_in_entry(v.handle(), &builder, &mut cx, code);
        code.write_ln(format!("frag_out{i} = {root};"));
      });
  });
  cx.gen_fn_depends(&mut code);
  code.output()
}

fn gen_node_with_dep_in_entry(
  node: ShaderGraphNodeRawHandle,
  builder: &ShaderGraphBuilder,
  cx: &mut CodeGenCtx,
  code: &mut CodeBuilder,
) -> String {
  let root = builder.scopes.first().unwrap();
  let mut last = None;
  root.nodes.traverse_dfs_in_topological_order(
    node.handle,
    &mut |n| {
      let h = ShaderGraphNodeRawHandle {
        handle: n.handle(),
        graph_id: node.graph_id,
      };
      if cx.try_get_node_gen_result_var(h).is_none() {
        gen_node(n.data(), h, cx, code);
      }

      if let Some(name) = cx.try_get_node_gen_result_var(h) {
        last = name.to_owned().into();
      }
    },
    &mut || panic!("loop"),
  );
  last.unwrap()
}

fn gen_scope_full(scope: &ShaderGraphScope, cx: &mut CodeGenCtx, code: &mut CodeBuilder) {
  let nodes = &scope.nodes;
  cx.push_scope();
  scope
    .inserted
    .iter()
    .for_each(|n| gen_node(nodes.get_node(n.handle).data(), *n, cx, code));
  cx.pop_scope();
}

fn gen_node(
  data: &ShaderGraphNodeData,
  handle: ShaderGraphNodeRawHandle,
  cx: &mut CodeGenCtx,
  code: &mut CodeBuilder,
) {
  match &data.node {
    ShaderGraphNode::Write {
      source,
      target,
      implicit,
    } => {
      if *implicit {
        let name = cx.create_new_unique_name();
        code.write_ln(format!(
          "{} {} = {};",
          gen_type_impl(data.ty),
          name,
          cx.get_node_gen_result_var(*target)
        ));
        cx.top_scope_mut().code_gen_history.insert(
          handle,
          MiddleVariableCodeGenResult {
            var_name: name,
            statement: "".to_owned(),
          },
        );
      } else {
        let var_name = cx.get_node_gen_result_var(*target).to_owned();
        code.write_ln(format!(
          "{} = {};",
          cx.get_node_gen_result_var(*target),
          cx.get_node_gen_result_var(*source)
        ));
        cx.top_scope_mut().code_gen_history.insert(
          handle,
          MiddleVariableCodeGenResult {
            var_name,
            statement: "".to_owned(),
          },
        );
      }
      code
    }
    ShaderGraphNode::ControlFlow(cf) => {
      cx.top_scope_mut().code_gen_history.insert(
        handle,
        MiddleVariableCodeGenResult {
          var_name: "error_cf".to_owned(),
          statement: "".to_owned(),
        },
      );
      match cf {
        ShaderControlFlowNode::If { condition, scope } => {
          code
            .write_ln(format!(
              "if ({}) {{",
              cx.get_node_gen_result_var(*condition)
            ))
            .tab();

          gen_scope_full(scope, cx, code);

          code.un_tab().write_ln("}")
        }
        ShaderControlFlowNode::For {
          source,
          scope,
          iter,
        } => {
          let name = cx.get_node_gen_result_var(*iter);
          let head = match source {
            ShaderIteratorAble::Const(v) => {
              format!("for(int {name} = 0; {name} < {v}; {name}++) {{")
            }
            ShaderIteratorAble::Count(v) => format!(
              "for(int {name} = 0; {name} < {count}; {name}++) {{",
              count = cx.get_node_gen_result_var(v.handle())
            ),
          };
          code.write_ln(head).tab();

          gen_scope_full(scope, cx, code);

          code.un_tab().write_ln("}")
        }
      }
    }
    ShaderGraphNode::SideEffect(effect) => match effect {
      ShaderSideEffectNode::Continue => code.write_ln("continue;"),
      ShaderSideEffectNode::Break => code.write_ln("break;"),
      ShaderSideEffectNode::Return(v) => {
        code.write_ln(format!("return {};", cx.get_node_gen_result_var(*v)))
      }
      ShaderSideEffectNode::Termination => code.write_ln("discard;"),
    },
    ShaderGraphNode::Input(input) => {
      cx.top_scope_mut().code_gen_history.insert(
        handle,
        MiddleVariableCodeGenResult {
          var_name: gen_input_name(input),
          statement: "".to_owned(),
        },
      );
      code
    }
    ShaderGraphNode::UnNamed => {
      let var_name = cx.create_new_unique_name();
      cx.top_scope_mut().code_gen_history.insert(
        handle,
        MiddleVariableCodeGenResult {
          var_name,
          statement: "".to_owned(),
        },
      );
      code
    }
    ShaderGraphNode::Expr(expr) => {
      let name = cx.create_new_unique_name();
      let expr = gen_expr(expr, cx);
      let ty = gen_type_impl(data.ty);
      let statement = format!("{ty} {name} = {expr};");
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

fn gen_expr(data: &ShaderGraphNodeExpr, cx: &mut CodeGenCtx) -> String {
  match data {
    ShaderGraphNodeExpr::FunctionCall {
      meta: prototype,
      parameters,
    } => {
      cx.add_fn_dep(prototype);
      format!(
        "{}({})",
        prototype.function_name,
        parameters
          .iter()
          .map(|from| { cx.get_node_gen_result_var(*from) })
          .collect::<Vec<_>>()
          .join(", ")
      )
    }
    ShaderGraphNodeExpr::SamplerCombinedTextureSampling { texture, position } => {
      let combined = cx.get_node_gen_result_var(*texture);
      format!(
        "textureSample({}, {})",
        combined,
        cx.get_node_gen_result_var(*position),
      )
    }
    ShaderGraphNodeExpr::TextureSampling { .. } => {
      unreachable!("can not use standalone sampling in glsl es")
    }
    ShaderGraphNodeExpr::Swizzle { ty, source } => {
      format!("{}.{}", cx.get_node_gen_result_var(*source), ty)
    }
    ShaderGraphNodeExpr::Operator(o) => match o {
      OperatorNode::Unary { one, operator } => {
        let op = match operator {
          UnaryOperator::LogicalNot => "!",
        };
        let one = cx.get_node_gen_result_var(*one);
        format!("{}{}", op, one)
      }
      OperatorNode::Binary {
        left,
        right,
        operator,
      } => {
        let op = match operator {
          BinaryOperator::Add => "+",
          BinaryOperator::Sub => "-",
          BinaryOperator::Mul => "*",
          BinaryOperator::Div => "/",
          BinaryOperator::Eq => "==",
          BinaryOperator::NotEq => "!=",
          BinaryOperator::GreaterThan => ">",
          BinaryOperator::LessThan => "<",
          BinaryOperator::GreaterEqualThan => ">=",
          BinaryOperator::LessEqualThan => "<=",
          BinaryOperator::LogicalOr => "||",
          BinaryOperator::LogicalAnd => "&&",
        };
        let left = cx.get_node_gen_result_var(*left);
        let right = cx.get_node_gen_result_var(*right);
        format!("{} {} {}", left, op, right)
      }
    },
    ShaderGraphNodeExpr::FieldGet {
      field_name,
      struct_node,
    } => format!(
      "{}.{}",
      cx.get_node_gen_result_var(*struct_node),
      field_name
    ),
    ShaderGraphNodeExpr::StructConstruct { meta, fields } => {
      format!(
        "{}({})",
        meta.name,
        fields
          .iter()
          .map(|from| { cx.get_node_gen_result_var(*from) })
          .collect::<Vec<_>>()
          .join(", ")
      )
    }
    ShaderGraphNodeExpr::Const(ConstNode { data }) => gen_primitive_literal(*data),
    ShaderGraphNodeExpr::Copy(node) => cx.get_node_gen_result_var(*node).to_owned(),
    ShaderGraphNodeExpr::Compose { target, parameters } => {
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
    ShaderGraphNodeExpr::MatInverse(n) => format!("inverse({})", cx.get_node_gen_result_var(*n)),
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
    .for_each(|(_, &meta)| gen_struct(code, &meta.to_owned()))
}

fn gen_struct(builder: &mut CodeBuilder, meta: &ShaderStructMetaInfoOwned) {
  builder.write_ln(format!("struct {} {{", meta.name));
  builder.tab();
  for ShaderStructFieldMetaInfoOwned { name, ty, .. } in &meta.fields {
    builder.write_ln(format!("{}: {};", gen_fix_type_impl(*ty), name,));
  }
  builder.un_tab();
  builder.write_ln("};");
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
      let mut item_index = 0;
      b.bindings.iter().for_each(|(ty, vis)| {
        if vis.get().is_visible_to(stage) {
          code.write_ln(format!(
            "uniform {} uniform_b_{}_i_{};",
            gen_type_impl(*ty),
            group_index,
            item_index,
          ));
          item_index += 1;
        }
      });
    })
}

fn gen_entry(code: &mut CodeBuilder, _: ShaderStages, mut content: impl FnMut(&mut CodeBuilder)) {
  code.write_ln("void main() {").tab();
  content(code);
  code.un_tab().write_ln("}");
}

fn gen_primitive_type(ty: PrimitiveShaderValueType) -> &'static str {
  match ty {
    PrimitiveShaderValueType::Float32 => "float",
    PrimitiveShaderValueType::Vec2Float32 => "vec2",
    PrimitiveShaderValueType::Vec3Float32 => "vec3",
    PrimitiveShaderValueType::Vec4Float32 => "vec4",
    PrimitiveShaderValueType::Mat2Float32 => "mat2",
    PrimitiveShaderValueType::Mat3Float32 => "mat3",
    PrimitiveShaderValueType::Mat4Float32 => "mat4",
    PrimitiveShaderValueType::Uint32 => "uint",
    PrimitiveShaderValueType::Bool => "bool",
  }
}

fn gen_type_impl(ty: ShaderValueType) -> String {
  match ty {
    ShaderValueType::Sampler => unreachable!("unable to use standalone sampler in glsl es target"),
    ShaderValueType::Texture => unreachable!("unable to use standalone sampler in glsl es target"),
    ShaderValueType::Fixed(ty) => gen_fix_type_impl(ty).to_owned(),
    ShaderValueType::Never => unreachable!("can not code generate never type"),
    ShaderValueType::SamplerCombinedTexture => "sampler2D".to_owned(),
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
    ShaderBuiltIn::VertexIndexId => "gl_VertexId",
    ShaderBuiltIn::VertexInstanceId => "gl_InstanceID",
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
