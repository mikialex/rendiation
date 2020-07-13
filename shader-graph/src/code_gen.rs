use crate::*;
use std::collections::HashMap;

struct CodeGenCtx {
  var_guid: usize,
  code_gen_history: HashMap<ShaderGraphNodeHandleUntyped, MiddleVariableCodeGenResult>,
}

struct MiddleVariableCodeGenResult{
  ref_node: ShaderGraphNodeHandleUntyped,
  var_guid: usize,
  expression_str: String,
}

impl CodeGenCtx {
  // fn get_unique_var_name(&mut self) -> String {
  //   String::from("v") + self.var_guid.to_string().as_ref()
  // }
}

impl ShaderGraph {
  fn gen_link_code(&self, handle: ShaderGraphNodeHandleUntyped) -> String {
    let depencyList = self.nodes.topological_order_list(handle);
    todo!()
  }
}

// trait NodeCodeGenBehaviour {
//   fn should_inline() -> bool;
//   fn code_gen(&self, graph: &ShaderGraph, result: &mut String);
// }


// impl NodeCodeGenBehaviour for UniformNode {
//   fn should_inline() -> bool{ true }
//   fn code_gen(&self, _: &ShaderGraph, result: &mut String){
//     result += "\n"
//   }
// }