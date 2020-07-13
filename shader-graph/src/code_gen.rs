use crate::*;
use std::collections::HashMap;

struct CodeGenCtx {
  var_guid: usize,
  code_gen_history: HashMap<ShaderGraphNodeHandleUntyped, MiddleVariableCodeGenResult>,
}

impl CodeGenCtx {
  pub fn add_node_result(&mut self, mut result: MiddleVariableCodeGenResult) {
    result.var_guid = self.var_guid;
    self.var_guid += 1;
    self.code_gen_history.insert(result.ref_node, result);
  }
}

struct MiddleVariableCodeGenResult {
  ref_node: ShaderGraphNodeHandleUntyped,
  var_guid: usize,
  expression_str: String,
}

impl MiddleVariableCodeGenResult {
  fn new(ref_node: ShaderGraphNodeHandleUntyped, expression_str: String) -> Self {
    Self {
      ref_node,
      var_guid: 0, // this will initialize in code gen ctx
      expression_str,
    }
  }
}

impl CodeGenCtx {
  // fn get_unique_var_name(&mut self) -> String {
  //   String::from("v") + self.var_guid.to_string().as_ref()
  // }
}

fn gen_function_node_exp(node: &FunctionNode, ctx: &CodeGenCtx) -> String {
  todo!()
}

impl ShaderGraph {
  fn gen_code_node(&self, handle: ShaderGraphNodeHandleUntyped, ctx: &mut CodeGenCtx) -> String {
    let mut builder = CodeBuilder::new();
    builder.write_ln("void main() {").tab();

    let dependency_list = self.nodes.topological_order_list(handle);
    dependency_list.iter().for_each(|&h| {
      let node = self.nodes.get_node(h).data();
      use ShaderGraphNodeData::*;
      match &node.data {
        Function(node) => {}
        Input(node) => ctx.add_node_result(MiddleVariableCodeGenResult::new(h, node.name.clone())),
      }
    });
    
    builder.un_tab().write_ln("}");

    todo!()
  }

  pub fn gen_code_vertex(&self) -> String {
    let mut ctx = CodeGenCtx {
      var_guid: 0,
      code_gen_history: HashMap::new(),
    };
    todo!();
  }
}

struct CodeBuilder {
  tab: String,
  tab_state: usize,
  str: String,
}

impl CodeBuilder {
  pub fn new() -> Self {
    Self {
      tab: String::from("  "),
      tab_state: 0,
      str: String::new(),
    }
  }
  pub fn tab(&mut self) -> &mut Self {
    self.tab_state += 1;
    self
  }
  pub fn un_tab(&mut self) -> &mut Self {
    self.tab_state -= 1;
    self
  }
  pub fn write_ln(&mut self, content: &str) -> &mut Self {
    self.str.push_str("\n");
    (0..self.tab_state).for_each(|_| self.str.push_str(&self.tab));
    self.str.push_str(content);
    self
  }
  pub fn output(self) -> String {
    self.str
  }
}
