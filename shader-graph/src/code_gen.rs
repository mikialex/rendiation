use crate::code_builder::*;
use crate::*;
use std::collections::HashMap;

struct CodeGenCtx {
  var_guid: usize,
  code_gen_history: HashMap<ShaderGraphNodeHandleUntyped, MiddleVariableCodeGenResult>,
}

impl CodeGenCtx {
  pub fn add_node_result(&mut self, mut result: MiddleVariableCodeGenResult) -> &str {
    result.var_name = format!("{}", self.var_guid);
    self.var_guid += 1;
    self
      .code_gen_history
      .entry(result.ref_node)
      .or_insert(result)
      .expression_str
      .as_str()
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
      .iter()
      .for_each(|&h| {
        // this node has generated, skip
        if ctx.code_gen_history.contains_key(&h) {
          return;
        }

        let node_wrap = self.nodes.get_node(h);
        use ShaderGraphNodeData::*;
        let result = match &node_wrap.data().data {
          Function(_) => ctx.add_node_result(MiddleVariableCodeGenResult::new(
            h,
            node_wrap
              .from()
              .iter()
              .map(|from| ctx.code_gen_history.get(from).unwrap().var_name.as_str())
              .collect::<Vec<_>>()
              .join(", "),
          )),
          Input(node) => {
            ctx.add_node_result(MiddleVariableCodeGenResult::new(h, node.name.clone()))
          }
        };
        builder.write_ln(result);
      });
  }

  pub fn gen_code_vertex(&self) -> String {
    let mut builder = CodeBuilder::new();
    builder.write_ln("void main() {").tab();

    let mut ctx = CodeGenCtx {
      var_guid: 0,
      code_gen_history: HashMap::new(),
    };

    self.varyings.iter().for_each(|&v| {
      self.gen_code_node(v, &mut ctx, &mut builder);
    });

    self.gen_code_node(
      self.vertex_position.expect("vertex position not set"),
      &mut ctx,
      &mut builder,
    );

    builder.write_ln("").un_tab().write_ln("}");
    builder.output()
  }
}
