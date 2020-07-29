mod code_builder;
use crate::*;
use code_builder::CodeBuilder;
use std::collections::HashMap;

struct CodeGenCtx {
  var_guid: usize,
  code_gen_history: HashMap<ShaderGraphNodeHandleUntyped, MiddleVariableCodeGenResult>,
  depend_functions: HashSet<Arc<ShaderFunction>>,
}

impl CodeGenCtx {
  fn new() -> Self {
    Self {
      var_guid: 0,
      code_gen_history: HashMap::new(),
      depend_functions: HashSet::new(),
    }
  }

  fn add_node_result(&mut self, mut result: MiddleVariableCodeGenResult) -> &str {
    result.var_name = format!("{}", self.var_guid);
    self.var_guid += 1;
    self
      .code_gen_history
      .entry(result.ref_node)
      .or_insert(result)
      .expression_str
      .as_str()
  }

  fn add_fn_dep(&mut self, node: &FunctionNode) {
    self.depend_functions.insert(node.prototype.clone());
  }

  fn gen_fn_depends(&self) -> String {
    let mut builder = CodeBuilder::new();
    let mut resolved_fn = HashSet::new();
    self.depend_functions.iter().for_each(|f| {
      if f.depend_functions.len() == 0 {
        builder.write_ln("").write_raw(f.function_source);
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
            builder.write_ln("").write_raw(f.function_source);
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
  ref_node: ShaderGraphNodeHandleUntyped,
  var_name: String,
  expression_str: String,
}

impl MiddleVariableCodeGenResult {
  fn new(ref_node: ShaderGraphNodeHandleUntyped, expression_str: String) -> Self {
    Self {
      ref_node,
      var_name: String::new(), // this will initialize in code gen ctx
      expression_str,
    }
  }
}

impl ShaderGraph {
  fn gen_code_node(
    &self,
    handle: ShaderGraphNodeHandleUntyped,
    ctx: &mut CodeGenCtx,
    builder: &mut CodeBuilder,
  ) {
    builder.write_ln("");

    self
      .nodes
      .topological_order_list(handle)
      .unwrap()
      .iter()
      .for_each(|&h| {
        // this node has generated, skip
        if ctx.code_gen_history.contains_key(&h) {
          return;
        }

        let node_wrap = self.nodes.get_node(h);
        use ShaderGraphNodeData::*;
        let result = match &node_wrap.data().data {
          Function(n) => {
            ctx.add_fn_dep(n);
            ctx.add_node_result(MiddleVariableCodeGenResult::new(
              h,
              node_wrap
                .from()
                .iter()
                .map(|from| ctx.code_gen_history.get(from).unwrap().var_name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            ))
          }
          Input(node) => {
            ctx.add_node_result(MiddleVariableCodeGenResult::new(h, node.name.clone()))
          }
        };
        builder.write_ln(result);
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
      self.vertex_position.expect("vertex position not set"),
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

  fn gen_header_vert(&self) -> String {
    let mut result = String::new();

    // attributes
    result += self
      .attributes
      .iter()
      .map(|a| {
        let info = self.nodes.get_node(a.0).data();
        let input = info.unwrap_as_input();
        format!(
          "layout(location = {}) in {} {};",
          a.1,
          self.type_id_map.get(&info.node_type).unwrap(),
          input.name
        )
      })
      .collect::<Vec<String>>()
      .join("\n")
      .as_ref();

    result
  }

  fn gen_header_frag(&self) -> String {
    let mut result = String::new();

    // varyings
    result += self
      .varyings
      .iter()
      .map(|a| {
        let info = self.nodes.get_node(a.0).data();
        let input = info.unwrap_as_input();
        format!(
          "layout(location = {}) in {} {};",
          a.1,
          self.type_id_map.get(&info.node_type).unwrap(),
          input.name
        )
      })
      .collect::<Vec<String>>()
      .join("\n")
      .as_ref();

    result
  }
}

impl ShaderGraphBindGroup {
  pub fn gen_header(&self, graph: &ShaderGraph) -> String {
    let result = String::new();
    result
  }
}
