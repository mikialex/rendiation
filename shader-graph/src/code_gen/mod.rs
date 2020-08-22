mod code_builder;
use crate::*;
use code_builder::CodeBuilder;
use std::{collections::HashMap, fmt::Display};

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

  fn add_node_result(
    &mut self,
    mut result: MiddleVariableCodeGenResult,
  ) -> &MiddleVariableCodeGenResult {
    result.var_name = format!("temp{}", self.var_guid);
    self.var_guid += 1;
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
      if f.depend_functions.len() == 0 {
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
  ref_node: ShaderGraphNodeHandleUntyped,
  type_name: &'static str,
  var_name: String,
  expression_str: String,
}

impl MiddleVariableCodeGenResult {
  fn new(
    ref_node: ShaderGraphNodeHandleUntyped,
    expression_str: String,
    graph: &ShaderGraph,
  ) -> Self {
    let info = graph.nodes.get_node(ref_node).data();
    Self {
      type_name: graph.type_id_map.get(&info.node_type).unwrap(),
      ref_node,
      var_name: String::new(), // this will initialize in code gen ctx
      expression_str,
    }
  }
}

impl Display for MiddleVariableCodeGenResult {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{} {} = {};",
      self.type_name, self.var_name, self.expression_str
    )
  }
}

impl ShaderGraph {
  fn gen_code_node(
    &self,
    handle: ShaderGraphNodeHandleUntyped,
    ctx: &mut CodeGenCtx,
    builder: &mut CodeBuilder,
    assign_name: &str,
  ) {
    builder.write_ln("");

    let depends = self.nodes.topological_order_list(handle).unwrap();

    depends.iter().enumerate().for_each(|(i, &h)| {
      // this node has generated, skip
      if ctx.code_gen_history.contains_key(&h) {
        return;
      }

      let node_wrap = self.nodes.get_node(h);
      use ShaderGraphNodeData::*;
      let result = match &node_wrap.data().data {
        Function(n) => {
          ctx.add_fn_dep(n);
          let fn_call = format!(
            "{}({})",
            n.prototype.function_name,
            node_wrap
              .from()
              .iter()
              .map(|from| ctx.code_gen_history.get(from).unwrap().var_name.as_str())
              .collect::<Vec<_>>()
              .join(", ")
          );
          ctx.add_node_result(MiddleVariableCodeGenResult::new(h, fn_call, self))
        }
        Input(node) => {
          ctx.add_node_result(MiddleVariableCodeGenResult::new(h, node.name.clone(), self))
        }
        Output((i, _)) => ctx.add_node_result(MiddleVariableCodeGenResult::new(
          h,
          format!("vary{}", i),
          self,
        )),
      };

      builder.write_ln(&format!("{}", result));

      if i == depends.len() - 1 {
        // the last one should extra output
        builder.write_ln(&format!("{} = {}", assign_name, result.var_name));
      }
    });
  }

  pub fn gen_code_vertex(&self) -> String {
    let mut ctx = CodeGenCtx::new();
    let mut builder = CodeBuilder::new();
    builder.write_ln("void main() {").tab();

    self.varyings.iter().for_each(|&v| {
      self.gen_code_node(v.0, &mut ctx, &mut builder, &format!("vary{}", v.1));
    });

    self.gen_code_node(
      unsafe {
        self
          .vertex_position
          .expect("vertex position not set")
          .cast_type()
      },
      &mut ctx,
      &mut builder,
      "gl_Position",
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
      self.gen_code_node(v.0, &mut ctx, &mut builder, &format!("output{}", v.1));
    });

    builder.write_ln("").un_tab().write_ln("}");

    let header = self.gen_header_frag();
    let main = builder.output();
    let lib = ctx.gen_fn_depends();
    header + "\n" + &lib + "\n" + &main
  }

  fn gen_header_vert(&self) -> String {
    let mut result = String::from("#version 450\n");

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
          input.name.as_str()
        )
      })
      .collect::<Vec<String>>()
      .join("\n")
      .as_ref();

    result += self.gen_bindgroups_header(ShaderStage::Vertex).as_str();

    result
  }

  fn gen_bindgroups_header(&self, stage: ShaderStage) -> String {
    self
      .bindgroups
      .iter()
      .enumerate()
      .map(|(i, b)| b.gen_header(self, i, stage))
      .collect::<Vec<_>>()
      .join("\n")
  }

  fn gen_header_frag(&self) -> String {
    let mut result = String::from("#version 450\n");

    result += self.gen_bindgroups_header(ShaderStage::Fragment).as_str();

    // varyings
    result += self
      .varyings
      .iter()
      .map(|a| {
        let info = self.nodes.get_node(a.0).data();
        // let id = info.unwrap_as_vary();
        format!(
          "layout(location = {}) in {} {};",
          a.1,
          self.type_id_map.get(&info.node_type).unwrap(),
          format!("vary{}", a.1)
        )
      })
      .collect::<Vec<String>>()
      .join("\n")
      .as_ref();

    result
  }
}

impl ShaderGraphBindGroup {
  pub fn gen_header(&self, graph: &ShaderGraph, index: usize, stage: ShaderStage) -> String {
    self
      .inputs
      .iter()
      .enumerate()
      .filter_map(|(i, h)| {
        if stage != h.1 {
          return None;
        }
        match &h.0 {
          ShaderGraphUniformInputType::NoneUBO(node) => {
            let info = graph.nodes.get_node(*node).data();
            let input = info.unwrap_as_input();
            Some(format!(
              "layout(set = {}, binding = {}) uniform {} {};\n",
              index,
              i,
              graph.type_id_map.get(&info.node_type).unwrap(),
              input.name.as_str()
            ))
          }
          ShaderGraphUniformInputType::UBO((info, _)) => Some(format!(
            "layout(set = {}, binding = {}) {};",
            index, i, info.code_cache
          )),
        }
      })
      .collect::<Vec<_>>()
      .join("\n")
  }
}
