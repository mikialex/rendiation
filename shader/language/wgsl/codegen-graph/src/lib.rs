#![feature(let_chains)]

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

  gen_uniform_structs(&mut code, &builder);
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

      vertex
        .vertex_out
        .iter()
        .for_each(|(_, VertexIOInfo { node, location, .. })| {
          let root = gen_node_with_dep_in_entry(node.handle(), &builder, &mut cx, code);
          code.write_ln(format!("out.vertex_out{location} = {root};"));
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
  gen_uniform_structs(&mut code, &builder);
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
  code.write_ln("@builtin(vertex_index) bt_vertex_vertex_id: u32,");
  code.write_ln("@builtin(instance_index) bt_vertex_instance_id: u32,");
  vertex
    .vertex_in
    .iter()
    .for_each(|(_, VertexIOInfo { ty, location, .. })| {
      code.write_ln(format!(
        "@location({location}) vertex_in_{location}: {},",
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

  vertex
    .vertex_out
    .iter()
    .for_each(|(_, VertexIOInfo { ty, location, .. })| {
      shader_struct.fields.push(ShaderStructFieldMetaInfoOwned {
        name: format!("vertex_out{location}"),
        ty: ShaderStructMemberValueType::Primitive(*ty),
        ty_deco: ShaderFieldDecorator::Location(*location).into(),
      });
    });

  gen_struct(code, &shader_struct, false);
}

fn _gen_interpolation(int: ShaderVaryingInterpolation) -> &'static str {
  match int {
    ShaderVaryingInterpolation::Flat => "flat",
    ShaderVaryingInterpolation::Perspective => "perspective",
  }
}

fn gen_fragment_in_declare(code: &mut CodeBuilder, frag: &ShaderGraphFragmentBuilder) {
  frag.fragment_in.iter().for_each(|(_, (_, ty, _int, i))| {
    // code.write_ln(format!(
    //   "@location({i}) @interpolate({}, center) fragment_in_{i}: {}",
    //   gen_interpolation(*int),
    //   gen_primitive_type(*ty)
    // ));
    code.write_ln(format!(
      "@location({i}) fragment_in_{i}: {},",
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

  gen_struct(code, &shader_struct, false);
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
      index,
    } => format!(
      "textureSample({}, {}, {}{}{})",
      cx.get_node_gen_result_var(*texture),
      cx.get_node_gen_result_var(*sampler),
      cx.get_node_gen_result_var(*position),
      index.map(|_| ", ").unwrap_or(""), // naga's parser is not handling optional ","
      index.map(|i| cx.get_node_gen_result_var(i)).unwrap_or(""),
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
    ShaderGraphNodeExpr::MatShrink { source, dimension } => {
      let from = cx.get_node_gen_result_var(*source);
      // wgsl is terrible!
      // todo, support node -> type and check;
      // todo, we only support 4 -> 3 now;
      // todo, check self if 4
      assert_eq!(*dimension, 3);
      format!("mat3x3({from}[0].xyz, {from}[1].xyz, {from}[2].xyz)")
    }
    ShaderGraphNodeExpr::MatInverse(n) => format!("inverse({})", cx.get_node_gen_result_var(*n)),
    ShaderGraphNodeExpr::MatTranspose(n) => {
      format!("transpose({})", cx.get_node_gen_result_var(*n))
    }
  }
}

fn gen_input_name(input: &ShaderGraphInputNode) -> String {
  match input {
    ShaderGraphInputNode::BuiltIn(ty) => gen_built_in(*ty).to_owned(),
    ShaderGraphInputNode::Uniform {
      bindgroup_index,
      entry_index,
    } => format!("uniform_b_{}_i_{}", bindgroup_index, entry_index),
    ShaderGraphInputNode::VertexIn {
      location: index, ..
    } => format!("vertex_in_{}", index),
    ShaderGraphInputNode::FragmentIn {
      location: index, ..
    } => format!("fragment_in_{}", index),
  }
}

fn gen_uniform_structs(code: &mut CodeBuilder, builder: &ShaderGraphBuilder) {
  builder
    .struct_defines
    .iter()
    .for_each(|&meta| gen_struct(code, &meta.to_owned(), true))
}

/// The shadergraph struct not mark any alignment info (as same as glsl)
/// but the wgsl requires explicit alignment and size mark, so we have to generate these.
///
/// When some struct requires 140 layout, we can assure all the type the struct used is also
/// std140, this is statically constraint by traits. That means if we generate all uniform
/// structs and it's dependency struct first, the following struct  and it's dependency struct
/// we meet in function dependency collecting can regard them as pure shader class. This also
/// applied to future 430 layout support in future.
///
fn gen_struct(builder: &mut CodeBuilder, meta: &ShaderStructMetaInfoOwned, is_uniform: bool) {
  builder.write_ln(format!("struct {} {{", meta.name));
  builder.tab();

  if is_uniform {
    let mut current_byte_used = 0;
    let mut previous: Option<&ShaderStructMemberValueType> = None;
    for ShaderStructFieldMetaInfoOwned { name, ty, ty_deco } in &meta.fields {
      let explicit_align = None;
      if let Some(previous) = previous {
        let previous_align_require = previous.align_of_self(StructLayoutTarget::Std140);
        if current_byte_used % previous_align_require != 0 {
          explicit_align = previous_align_require.into();
        }
      };

      let explicit_align = explicit_align
        .map(|a| format!("align {}", a))
        .unwrap_or(format!(""));

      builder.write_ln(format!(
        "{} {}: {},",
        explicit_align,
        name,
        gen_fix_type_impl(*ty)
      ));

      current_byte_used += ty.size_of_self(StructLayoutTarget::Std430);
      previous = Some(ty)
    }
  } else {
    for ShaderStructFieldMetaInfoOwned { name, ty, ty_deco } in &meta.fields {
      let built_in_deco = if let Some(ty_deco) = ty_deco {
        match ty_deco {
          ShaderFieldDecorator::BuiltIn(built_in) => format!(
            "@builtin({})",
            match built_in {
              ShaderBuiltInDecorator::VertexIndex => "vertex_index",
              ShaderBuiltInDecorator::InstanceIndex => "instance_index",
              ShaderBuiltInDecorator::VertexPositionOut => "position",
              ShaderBuiltInDecorator::FragmentPositionIn => "position",
            }
          ),
          ShaderFieldDecorator::Location(location) => format!("@location({location})"),
        }
      } else {
        "".to_owned()
      };

      builder.write_ln(format!(
        "{} {}: {},",
        built_in_deco,
        name,
        gen_fix_type_impl(*ty)
      ));
    }
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
                sample_type: TextureSampleType::Float { filterable: true },
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
      "@group({}) @binding({}) var{} uniform_b_{}_i_{}: {};",
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
  }
  *item_index += 1;
}

fn gen_entry(
  code: &mut CodeBuilder,
  stage: ShaderStages,
  mut content: impl FnMut(&mut CodeBuilder),
  mut parameter: impl FnMut(&mut CodeBuilder),
  mut return_type: impl FnMut(&mut CodeBuilder),
) {
  let stage_name = match stage {
    ShaderStages::Vertex => "vertex",
    ShaderStages::Fragment => "fragment",
  };

  code.write_ln(format!("@{stage_name}"));
  code.write_ln(format!("fn {stage_name}_main(")).tab();
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
    ShaderValueType::Texture {
      dimension,
      sample_type,
    } => {
      let suffix = match sample_type {
        TextureSampleType::Float { .. } => "",
        TextureSampleType::Depth => "_depth",
        TextureSampleType::Sint => todo!(),
        TextureSampleType::Uint => todo!(),
      };
      match dimension {
        TextureViewDimension::D1 => format!("texture{suffix}_1d<f32>"),
        TextureViewDimension::D2 => format!("texture{suffix}_2d<f32>"),
        TextureViewDimension::D2Array => format!("texture{suffix}_2d_array<f32>"),
        TextureViewDimension::Cube => format!("texture{suffix}_cube<f32>"),
        TextureViewDimension::CubeArray => format!("texture{suffix}_cube_array<f32>"),
        TextureViewDimension::D3 => format!("texture{suffix}_3d<f32>"),
      }
    }
    ShaderValueType::Fixed(ty) => gen_fix_type_impl(ty),
    ShaderValueType::Never => unreachable!("can not code generate never type"),
    ShaderValueType::SamplerCombinedTexture => {
      unreachable!("combined sampler texture should handled above")
    }
  }
}

fn gen_fix_type_impl(ty: ShaderStructMemberValueType) -> String {
  match ty {
    ShaderStructMemberValueType::Primitive(ty) => gen_primitive_type(ty).to_owned(),
    ShaderStructMemberValueType::Struct(meta) => meta.name.to_owned(),
    ShaderStructMemberValueType::FixedSizeArray((ty, length)) => {
      format!("array<{}, {}>", gen_fix_type_impl(*ty), length)
    }
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
    PrimitiveShaderValue::Int32(v) => format!("{}", v),
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
