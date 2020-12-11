mod code_builder;
use crate::*;
use code_builder::CodeBuilder;
use std::{collections::HashMap, fmt::Display};
mod header;

struct CodeGenCtx {
  var_guid: usize,
  code_gen_history: HashMap<ShaderGraphNodeRawHandleUntyped, MiddleVariableCodeGenResult>,
  depend_functions: HashSet<&'static ShaderFunctionMetaInfo>,
}

#[allow(clippy::clone_double_ref)]
impl CodeGenCtx {
  fn new() -> Self {
    Self {
      var_guid: 0,
      code_gen_history: HashMap::new(),
      depend_functions: HashSet::new(),
    }
  }

  fn create_new_temp_name(&mut self) -> String {
    self.var_guid += 1;
    format!("temp{}", self.var_guid)
  }

  fn add_node_result(
    &mut self,
    result: MiddleVariableCodeGenResult,
  ) -> &MiddleVariableCodeGenResult {
    self
      .code_gen_history
      .entry(result.ref_node)
      .or_insert(result)
  }

  fn add_fn_dep(&mut self, node: &FunctionNode) {
    self.depend_functions.insert(node.prototype.clone());
  }

  fn gen_fn_depends(&self) -> String {
    let mut builder = CodeBuilder::new();
    let mut resolved_fn = HashSet::new();
    self.depend_functions.iter().for_each(|f| {
      if f.depend_functions.is_empty() {
        f.function_source.map(|s| builder.write_ln("").write_raw(s));
        resolved_fn.insert(f.clone());
      }

      let mut fn_dep_graph = ArenaGraph::new();
      let mut resolving_fn = HashMap::new();
      let mut fn_to_expand = vec![f.clone()];

      while let Some(f) = fn_to_expand.pop() {
        let self_node_handle = *resolving_fn
          .entry(f.clone())
          .or_insert_with(|| fn_dep_graph.create_node(f.clone()));
        f.depend_functions.iter().for_each(|f_d| {
          let dep_node_handle = resolving_fn
            .entry(f_d.clone())
            .or_insert_with(|| fn_dep_graph.create_node(f_d.clone()));
          fn_dep_graph.connect_node(*dep_node_handle, self_node_handle);
        });
      }
      fn_dep_graph.traverse_dfs_in_topological_order(
        Handle::from_raw_parts(0, 0),
        &mut |n| {
          let f = n.data();
          if !resolved_fn.contains(f) {
            f.function_source.map(|s| builder.write_ln("").write_raw(s));
            resolved_fn.insert(f.clone());
          }
        },
        &mut || panic!("loop exist"),
      )
    });
    builder.output()
  }
}

struct MiddleVariableCodeGenResult {
  ref_node: ShaderGraphNodeRawHandleUntyped,
  type_name: &'static str,
  var_name: String,
  expression_str: String,
  is_builtin_target: bool,
}

impl MiddleVariableCodeGenResult {
  fn new(
    ref_node: ShaderGraphNodeRawHandleUntyped,
    var_name: String,
    expression_str: String,
    graph: &ShaderGraph,
    is_builtin_target: bool,
  ) -> Self {
    let info = graph.nodes.get_node(ref_node).data();
    Self {
      type_name: graph.type_id_map.get(&info.node_type).unwrap(),
      ref_node,
      var_name,
      expression_str,
      is_builtin_target,
    }
  }
}

impl Display for MiddleVariableCodeGenResult {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{} {} = {};",
      if !self.is_builtin_target {
        self.type_name
      } else {
        ""
      },
      self.var_name,
      self.expression_str
    )
  }
}

impl ShaderGraph {
  fn gen_code_node(&self, handle: NodeUntyped, ctx: &mut CodeGenCtx, builder: &mut CodeBuilder) {
    builder.write_ln("");

    let depends = self.nodes.topological_order_list(handle.handle).unwrap();

    depends.iter().for_each(|&h| {
      // this node has generated, skip
      if ctx.code_gen_history.contains_key(&h) {
        return;
      }

      let node_wrap = self.nodes.get_node(h);

      // None is input node, skip
      if let Some(result) = node_wrap.data().gen_node_record(h, self, ctx) {
        builder.write_ln(&format!("{}", result));
        ctx.add_node_result(result);
      }
    });
  }

  pub fn gen_code_vertex(&self) -> String {
    let mut ctx = CodeGenCtx::new();
    let mut builder = CodeBuilder::new();
    builder.write_ln("void main() {").tab();

    self.varyings.iter().for_each(|&v| {
      self.gen_code_node(v.0, &mut ctx, &mut builder);
    });

    self.gen_code_node(
      unsafe {
        self
          .vertex_position
          .expect("vertex position not set")
          .handle
          .cast_type()
          .into()
      },
      &mut ctx,
      &mut builder,
    );

    builder.write_ln("").un_tab().write_ln("}");

    let header = self.gen_header_vert();
    let main = builder.output();
    let lib = ctx.gen_fn_depends();
    header + "\n" + &lib + "\n" + &main
  }

  pub fn gen_code_frag(&self) -> String {
    let mut ctx = CodeGenCtx::new();
    let mut builder = CodeBuilder::new();
    builder.write_ln("void main() {").tab();

    self.frag_outputs.iter().for_each(|&v| {
      self.gen_code_node(v.0, &mut ctx, &mut builder);
    });

    builder.write_ln("").un_tab().write_ln("}");

    let header = self.gen_header_frag();
    let main = builder.output();
    let lib = ctx.gen_fn_depends();
    header + "\n" + &lib + "\n" + &main
  }
}

use ShaderGraphNodeData::*;
impl ShaderGraphNode<AnyType> {
  fn gen_node_record(
    &self,
    handle: ArenaGraphNodeHandle<Self>,
    graph: &ShaderGraph,
    ctx: &mut CodeGenCtx,
  ) -> Option<MiddleVariableCodeGenResult> {
    let node = graph.nodes.get_node(handle);
    if let Some((var_name, expression_str, is_builtin)) = self.gen_code_record_exp(node, graph, ctx)
    {
      Some(MiddleVariableCodeGenResult::new(
        handle,
        var_name,
        expression_str,
        graph,
        is_builtin,
      ))
    } else {
      None
    }
  }

  fn gen_code_record_exp(
    &self,
    node: &ArenaGraphNode<Self>,
    graph: &ShaderGraph,
    ctx: &mut CodeGenCtx,
  ) -> Option<(String, String, bool)> {
    match &self.data {
      Function(n) => {
        ctx.add_fn_dep(n);
        let fn_call = format!(
          "{}({})",
          n.prototype.function_name,
          node
            .from()
            .iter()
            .map(|from| { get_node_gen_result_var(*from, graph, ctx) })
            .collect::<Vec<_>>()
            .join(", ")
        );
        Some((ctx.create_new_temp_name(), fn_call, false))
      }
      BuiltInFunction(n) => {
        let fn_call = format!(
          "{}({})",
          n,
          node
            .from()
            .iter()
            .map(|from| { get_node_gen_result_var(*from, graph, ctx) })
            .collect::<Vec<_>>()
            .join(", ")
        );
        Some((ctx.create_new_temp_name(), fn_call, false))
      }
      Swizzle(s) => {
        let from = node.from().iter().next().unwrap();
        let from = get_node_gen_result_var(*from, graph, ctx);
        let swizzle_code = format!("{}.{}", from, s);
        Some((ctx.create_new_temp_name(), swizzle_code, false))
      }
      Operator(o) => {
        let left = get_node_gen_result_var(o.left, graph, ctx);
        let right = get_node_gen_result_var(o.right, graph, ctx);
        let code = format!("{} {} {}", left, o.operator, right);
        Some((ctx.create_new_temp_name(), code, false))
      }
      TextureSampling(n) => unsafe {
        let sampling_code = format!(
          "texture(sampler2D({}, {}), {})",
          get_node_gen_result_var(n.texture.cast_type(), graph, ctx),
          get_node_gen_result_var(n.sampler.cast_type(), graph, ctx),
          get_node_gen_result_var(n.position.cast_type(), graph, ctx),
        );
        Some((ctx.create_new_temp_name(), sampling_code, false))
      },
      Output(n) => {
        let from = node.from().iter().next().expect("output not set");
        Some((
          n.to_shader_var_name(),
          get_node_gen_result_var(*from, graph, ctx),
          true,
        ))
      }
      _ => None,
    }
  }
}

fn get_node_gen_result_var(
  node: ArenaGraphNodeHandle<ShaderGraphNodeUntyped>,
  graph: &ShaderGraph,
  ctx: &CodeGenCtx,
) -> String {
  let data = &graph.nodes.get_node(node).data().data;
  match data {
    Function(_) => ctx.code_gen_history.get(&node).unwrap().var_name.clone(),
    BuiltInFunction(_) => ctx.code_gen_history.get(&node).unwrap().var_name.clone(),
    TextureSampling(_) => ctx.code_gen_history.get(&node).unwrap().var_name.clone(),
    Swizzle(_) => ctx.code_gen_history.get(&node).unwrap().var_name.clone(),
    Operator(_) => ctx.code_gen_history.get(&node).unwrap().var_name.clone(),
    Input(n) => n.name.clone(),
    Output(n) => n.to_shader_var_name(),
    Const(value) => value.const_to_glsl(),
  }
}

impl ShaderGraphOutput {
  pub fn to_shader_var_name(&self) -> String {
    match self {
      Self::Vary(index) => format!("vary{}", index),
      Self::Frag(index) => format!("frag{}", index),
      Self::Vert => "gl_Position".to_owned(),
    }
  }
  pub fn is_builtin(&self) -> bool {
    matches!(self, Self::Vert)
  }
}
