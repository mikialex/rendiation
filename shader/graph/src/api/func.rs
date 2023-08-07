use crate::*;

pub fn begin_define_fn(name: String) -> Option<ShaderFunctionMetaInfo> {
  modify_graph(|g| g.begin_define_fn(name))
}

pub fn push_fn_parameter<T: ShaderGraphNodeType>() -> Node<T> {
  unsafe { modify_graph(|g| g.push_fn_parameter(T::TYPE)).into_node() }
}

pub fn end_fn_define(ty: Option<ShaderValueType>) -> ShaderFunctionMetaInfo {
  modify_graph(|g| g.end_fn_define(ty))
}

// I do this because I don't know how to destruct T from Node<T> in proc macro syc ast, sad!
pub trait ProcMacroNodeHelper {
  type NodeType;
}
impl<T> ProcMacroNodeHelper for Node<T> {
  type NodeType = T;
}

pub fn shader_fn_call(
  meta: ShaderFunctionMetaInfo,
  parameters: Vec<ShaderGraphNodeRawHandle>,
) -> ShaderGraphNodeRawHandle {
  modify_graph(|g| {
    let expr = ShaderGraphNodeExpr::FunctionCall {
      meta: ShaderFunctionType::Custom(todo!()),
      parameters,
    };
    g.make_expression(expr)
  })
}
