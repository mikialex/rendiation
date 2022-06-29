use shadergraph::*;

pub struct WGSL;

impl WGSL {
  pub fn vertex_entry_name(&self) -> &'static str {
    "vertex_main"
  }
  pub fn fragment_entry_name(&self) -> &'static str {
    "fragment_main"
  }
}

pub struct WGSLShaderSource {
  pub vertex: String,
  pub fragment: String,
}

impl ShaderGraphCodeGenTarget for WGSL {
  type ShaderSource = WGSLShaderSource;

  fn compile(
    &self,
    builder: &ShaderGraphRenderPipelineBuilder,
    vertex: ShaderGraphBuilder,
    fragment: ShaderGraphBuilder,
  ) -> Self::ShaderSource {
    let vertex = gen_vertex_shader(builder, vertex);
    let fragment = gen_fragment_shader(builder, fragment);
    WGSLShaderSource { vertex, fragment }
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
  gen_vertex_out_struct(&mut code, vertex);
  gen_bindings(
    &mut code,
    &pipeline_builder.bindgroups,
    ShaderStages::Vertex,
  );

  let mut code_entry = CodeBuilder::default();
  gen_entry(
    &mut code_entry,
    ShaderStages::Vertex,
    |code| {
      code.write_ln("var out: VertexOut;");

      if let Ok(position) = vertex.query::<ClipPosition>() {
        let root = gen_node_with_dep_in_entry(position.get().handle(), &builder, &mut cx, code);
        code.write_ln(format!("out.position = {root};"));
      }

      vertex.vertex_out.iter().for_each(|(_, (v, _, i))| {
        let root = gen_node_with_dep_in_entry(v.handle(), &builder, &mut cx, code);
        code.write_ln(format!("out.vertex_out{i} = {root};"));
      });
      code.write_ln("return out;");
    },
    |code| gen_vertex_in_declare(code, vertex),
    |code| {
      code.write_raw("VertexOut");
    },
  );
  cx.gen_fn_and_ty_depends(&mut code, gen_struct);
  code.output() + code_entry.output().as_str()
}

fn gen_fragment_shader(
  pipeline_builder: &ShaderGraphRenderPipelineBuilder,
  builder: ShaderGraphBuilder,
) -> String {
  let fragment = &pipeline_builder.fragment;

  let mut code = CodeBuilder::default();
  let mut cx = CodeGenCtx::default();
  gen_structs(&mut code, &builder);
  gen_fragment_out_struct(&mut code, fragment);
  gen_bindings(
    &mut code,
    &pipeline_builder.bindgroups,
    ShaderStages::Fragment,
  );

  let mut code_entry = CodeBuilder::default();
  gen_entry(
    &mut code_entry,
    ShaderStages::Fragment,
    |code| {
      code.write_ln("var out: FragmentOut;");
      fragment
        .frag_output
        .iter()
        .enumerate()
        .for_each(|(i, (v, _))| {
          let root = gen_node_with_dep_in_entry(v.handle(), &builder, &mut cx, code);
          code.write_ln(format!("out.frag_out{i} = {root};"));
        });
      code.write_ln("return out;");
    },
    |code| gen_fragment_in_declare(code, fragment),
    |code| {
      code.write_raw("FragmentOut");
    },
  );
  cx.gen_fn_and_ty_depends(&mut code, gen_struct);
  code.output() + code_entry.output().as_str()
}

fn gen_vertex_in_declare(code: &mut CodeBuilder, vertex: &ShaderGraphVertexBuilder) {
  code.write_ln("[[builtin(vertex_index)]] bt_vertex_vertex_id: u32,");
  code.write_ln("[[builtin(instance_index)]] bt_vertex_instance_id: u32,");
  vertex.vertex_in.iter().for_each(|(_, (_, ty, index))| {
    code.write_ln(format!(
      "[[location({index})]] vertex_in_{index}: {},",
      gen_primitive_type(*ty)
    ));
  })
}

fn gen_vertex_out_struct(code: &mut CodeBuilder, vertex: &ShaderGraphVertexBuilder) {
  let mut shader_struct = ShaderStructMetaInfoOwned::new("VertexOut");

  shader_struct.fields.push(ShaderStructFieldMetaInfoOwned {
    name: "position".into(),
    ty: ShaderStructMemberValueType::Primitive(PrimitiveShaderValueType::Vec4Float32),
    ty_deco: ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::VertexPositionOut).into(),
  });

  vertex.vertex_out.iter().for_each(|(_, (_, ty, i))| {
    shader_struct.fields.push(ShaderStructFieldMetaInfoOwned {
      name: format!("vertex_out{}", i),
      ty: ShaderStructMemberValueType::Primitive(*ty),
      ty_deco: ShaderFieldDecorator::Location(*i).into(),
    });
  });

  gen_struct(code, &shader_struct);
}

// fn gen_interpolation(int: ShaderVaryingInterpolation) -> &'static str {
//   match int {
//     ShaderVaryingInterpolation::Flat => "flat",
//     ShaderVaryingInterpolation::Perspective => "perspective",
//   }
// }

fn gen_fragment_in_declare(code: &mut CodeBuilder, frag: &ShaderGraphFragmentBuilder) {
  frag.fragment_in.iter().for_each(|(_, (_, ty, _int, i))| {
    // code.write_ln(format!(
    //   "[[location({i})]] [[interpolate({})]] fragment_in_{i}: {}",
    //   gen_interpolation(*int),
    //   gen_primitive_type(*ty)
    // ));
    code.write_ln(format!(
      "[[location({i})]] fragment_in_{i}: {},",
      gen_primitive_type(*ty)
    ));
  });
}

fn gen_fragment_out_struct(code: &mut CodeBuilder, frag: &ShaderGraphFragmentBuilder) {
  let mut shader_struct = ShaderStructMetaInfoOwned::new("FragmentOut");
  frag.frag_output.iter().enumerate().for_each(|(i, _)| {
    shader_struct.fields.push(ShaderStructFieldMetaInfoOwned {
      name: format!("frag_out{}", i),
      ty: ShaderStructMemberValueType::Primitive(PrimitiveShaderValueType::Vec4Float32),
      ty_deco: ShaderFieldDecorator::Location(i).into(),
    });
  });

  gen_struct(code, &shader_struct);
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
        gen_node(&n.data().node, h, cx, code);
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
    .for_each(|n| gen_node(&nodes.get_node(n.handle).data().node, *n, cx, code));
  cx.pop_scope();
}

fn gen_node(
  data: &ShaderGraphNode,
  handle: ShaderGraphNodeRawHandle,
  cx: &mut CodeGenCtx,
  code: &mut CodeBuilder,
) {
  match data {
    ShaderGraphNode::Write {
      source,
      target,
      implicit,
    } => {
      if *implicit {
        let name = cx.create_new_unique_name();
        code.write_ln(format!(
          "var {} = {};",
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
              format!("for(var {name}: i32 = 0; {name} < {v}; {name} = {name} + 1) {{")
            }
            ShaderIteratorAble::Count(v) => format!(
              "for(var {name}: i32 = 0; {name} < {count}; {name} = {name} + 1) {{",
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
      let statement = format!("var {name} = {expr};");
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

fn expand_combined(var: &str) -> (String, String) {
  (format!("{}_t", var), format!("{}_s", var))
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
      let (tex, sampler) = expand_combined(combined);
      format!(
        "textureSample({}, {}, {})",
        tex,
        sampler,
        cx.get_node_gen_result_var(*position),
      )
    }
    ShaderGraphNodeExpr::TextureSampling {
      texture,
      sampler,
      position,
    } => format!(
      "textureSample({}, {}, {})",
      cx.get_node_gen_result_var(*texture),
      cx.get_node_gen_result_var(*sampler),
      cx.get_node_gen_result_var(*position),
    ),
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
    .for_each(|&meta| gen_struct(code, &meta.to_owned()))
}

fn gen_struct(builder: &mut CodeBuilder, meta: &ShaderStructMetaInfoOwned) {
  builder.write_ln(format!("struct {} {{", meta.name));
  builder.tab();
  for ShaderStructFieldMetaInfoOwned { name, ty, ty_deco } in &meta.fields {
    let built_in_deco = if let Some(ty_deco) = ty_deco {
      match ty_deco {
        ShaderFieldDecorator::BuiltIn(built_in) => format!(
          "[[builtin({})]]",
          match built_in {
            ShaderBuiltInDecorator::VertexIndex => "vertex_index",
            ShaderBuiltInDecorator::InstanceIndex => "instance_index",
            ShaderBuiltInDecorator::VertexPositionOut => "position",
            ShaderBuiltInDecorator::FragmentPositionIn => "position",
          }
        ),
        ShaderFieldDecorator::Location(location) => format!("[[location({location})]]"),
      }
    } else {
      "".to_owned()
    };

    builder.write_ln(format!(
      "{} {}: {};",
      built_in_deco,
      name,
      gen_fix_type_impl(*ty)
    ));
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
      b.bindings.iter().for_each(|entry| match entry.0 {
        ShaderValueType::SamplerCombinedTexture => {
          gen_bind_entry(
            code,
            &(
              ShaderValueType::Texture {
                dimension: TextureViewDimension::D2,
              },
              entry.1.get(),
            ),
            group_index,
            &mut item_index,
            stage,
          );
          gen_bind_entry(
            code,
            &(ShaderValueType::Sampler, entry.1.get()),
            group_index,
            &mut item_index,
            stage,
          );
        }
        _ => {
          gen_bind_entry(
            code,
            &(entry.0, entry.1.get()),
            group_index,
            &mut item_index,
            stage,
          );
        }
      });
    })
}

fn gen_bind_entry(
  code: &mut CodeBuilder,
  entry: &(ShaderValueType, ShaderStageVisibility),
  group_index: usize,
  item_index: &mut usize,
  stage: ShaderStages,
) {
  if entry.1.is_visible_to(stage) {
    code.write_ln(format!(
      "[[group({}), binding({})]] var{} uniform_b_{}_i_{}: {};",
      group_index,
      item_index,
      match entry.0 {
        ShaderValueType::Fixed(_) => "<uniform>",
        _ => "",
      },
      group_index,
      item_index,
      gen_type_impl(entry.0),
    ));
    *item_index += 1;
  }
}

fn gen_entry(
  code: &mut CodeBuilder,
  stage: ShaderStages,
  mut content: impl FnMut(&mut CodeBuilder),
  mut parameter: impl FnMut(&mut CodeBuilder),
  mut return_type: impl FnMut(&mut CodeBuilder),
) {
  let name = match stage {
    ShaderStages::Vertex => "vertex",
    ShaderStages::Fragment => "fragment",
  };

  code.write_ln(format!("[[stage({name})]]"));
  code.write_ln(format!("fn {name}_main(")).tab();
  parameter(code);
  code.un_tab().write_ln(") ->");
  return_type(code);
  code.write_raw("{");
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
    PrimitiveShaderValueType::Int32 => "i32",
  }
}

fn gen_type_impl(ty: ShaderValueType) -> String {
  match ty {
    ShaderValueType::Sampler => "sampler".to_owned(),
    ShaderValueType::CompareSampler => "sampler_comparison".to_owned(),
    ShaderValueType::Texture { dimension } => match dimension {
      TextureViewDimension::D1 => "texture_1d<f32>".to_owned(),
      TextureViewDimension::D2 => "texture_2d<f32>".to_owned(),
      TextureViewDimension::D2Array => "texture_2d_array<f32>".to_owned(),
      TextureViewDimension::Cube => "texture_cube<f32>".to_owned(),
      TextureViewDimension::CubeArray => "texture_cube_array<f32>".to_owned(),
      TextureViewDimension::D3 => "texture_3d<f32>".to_owned(),
    },
    ShaderValueType::Fixed(ty) => gen_fix_type_impl(ty).to_owned(),
    ShaderValueType::Never => unreachable!("can not code generate never type"),
    ShaderValueType::SamplerCombinedTexture => {
      unreachable!("combined sampler texture should handled above")
    }
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
