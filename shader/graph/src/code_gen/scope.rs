use std::{
  collections::{HashMap, HashSet},
  fmt::Display,
};

use crate::*;

pub struct CodeGenScopeCtx {
  ctx_guid: usize,
  var_guid: usize,
  code_gen_history: HashMap<ShaderGraphNodeRawHandleUntyped, MiddleVariableCodeGenResult>,
}

#[allow(clippy::clone_double_ref)]
impl CodeGenScopeCtx {
  pub fn new(ctx_guid: usize) -> Self {
    Self {
      ctx_guid,
      var_guid: 0,
      code_gen_history: HashMap::new(),
    }
  }

  pub fn create_new_unique_name(&mut self) -> String {
    self.var_guid += 1;
    format!("v{}-{}", self.ctx_guid, self.var_guid)
  }

  pub fn write_node(&mut self, data: &ShaderGraphNodeData) -> &MiddleVariableCodeGenResult {
    todo!()
  }
}

pub struct MiddleVariableCodeGenResult {
  pub var_name: String,
  pub statement: String,
}

// use ShaderGraphNodeData::*;

// use super::CodeBuilder;
// impl ShaderGraphNode<AnyType> {
//   fn gen_node_record(
//     &self,
//     handle: ArenaGraphNodeHandle<Self>,
//     graph: &ShaderGraphShaderBuilder,
//     ctx: &mut CodeGenScopeCtx,
//   ) -> Option<MiddleVariableCodeGenResult> {
//     let node = graph.nodes.get_node(handle);
//     if let Some((var_name, expression_str, is_builtin)) = self.gen_code_record_exp(node, graph, ctx)
//     {
//       Some(MiddleVariableCodeGenResult::new(
//         handle,
//         var_name,
//         expression_str,
//         graph,
//         is_builtin,
//       ))
//     } else {
//       None
//     }
//   }

//   fn gen_code_record_exp(
//     &self,
//     node: &ArenaGraphNode<Self>,
//     graph: &ShaderGraphShaderBuilder,
//     ctx: &mut CodeGenScopeCtx,
//   ) -> Option<(String, String, bool)> {
//     match &self.data {
//       Function(n) => {
//         ctx.add_fn_dep(n);
//         let fn_call = format!(
//           "{}({})",
//           n.prototype.function_name,
//           n.parameters
//             .iter()
//             .map(|from| { get_node_gen_result_var(*from, graph, ctx) })
//             .collect::<Vec<_>>()
//             .join(", ")
//         );
//         Some((ctx.create_new_unique_name(), fn_call, false))
//       }
//       BuiltInFunction { parameters, name } => {
//         let fn_call = format!(
//           "{}({})",
//           name,
//           parameters
//             .iter()
//             .map(|from| { get_node_gen_result_var(*from, graph, ctx) })
//             .collect::<Vec<_>>()
//             .join(", ")
//         );
//         Some((ctx.create_new_unique_name(), fn_call, false))
//       }
//       Swizzle { ty, source } => {
//         let from = get_node_gen_result_var(*source, graph, ctx);
//         let swizzle_code = format!("{}.{}", from, ty);
//         Some((ctx.create_new_unique_name(), swizzle_code, false))
//       }
//       Operator(o) => {
//         let left = get_node_gen_result_var(o.left, graph, ctx);
//         let right = get_node_gen_result_var(o.right, graph, ctx);
//         let code = format!("{} {} {}", left, o.operator, right);
//         Some((ctx.create_new_unique_name(), code, false))
//       }
//       TextureSampling(n) => unsafe {
//         let sampling_code = format!(
//           "texture(sampler2D({}, {}), {})",
//           get_node_gen_result_var(n.texture.cast_type(), graph, ctx),
//           get_node_gen_result_var(n.sampler.cast_type(), graph, ctx),
//           get_node_gen_result_var(n.position.cast_type(), graph, ctx),
//         );
//         Some((ctx.create_new_unique_name(), sampling_code, false))
//       },
//       Output(n) => {
//         let from = node.from().iter().next().expect("output not set");
//         Some((
//           n.to_shader_var_name(),
//           get_node_gen_result_var(*from, graph, ctx),
//           true,
//         ))
//       }
//       _ => None,
//     }
//   }
// }

// fn get_node_gen_result_var(
//   node: ArenaGraphNodeHandle<ShaderGraphNodeUntyped>,
//   graph: &ShaderGraphShaderBuilder,
//   ctx: &CodeGenScopeCtx,
// ) -> String {
//   let data = &graph.nodes.get_node(node).data().data;
//   match data {
//     Function(_) => ctx.code_gen_history.get(&node).unwrap().var_name.clone(),
//     BuiltInFunction { .. } => ctx.code_gen_history.get(&node).unwrap().var_name.clone(),
//     TextureSampling(_) => ctx.code_gen_history.get(&node).unwrap().var_name.clone(),
//     Swizzle { .. } => ctx.code_gen_history.get(&node).unwrap().var_name.clone(),
//     Operator(_) => ctx.code_gen_history.get(&node).unwrap().var_name.clone(),
//     Input(n) => n.name.clone(),
//     Output(n) => n.to_shader_var_name(),
//     Const(ConstNode { data }) => data.const_to_glsl(),
//     FieldGet { .. } => todo!(),
//     StructConstruct { struct_id, fields } => todo!(),
//     Compose(_) => todo!(),
//     _ => todo!(),
//   }
// }

// impl ShaderGraphOutput {
//   pub fn to_shader_var_name(&self) -> String {
//     match self {
//       Self::Vary(index) => format!("vary{}", index),
//       Self::Frag(index) => format!("frag{}", index),
//       Self::Vert => "gl_Position".to_owned(),
//     }
//   }
//   pub fn is_builtin(&self) -> bool {
//     matches!(self, Self::Vert)
//   }
// }
