use rendiation_shadergraph::*;

pub static MAX_FUNCTION: once_cell::sync::Lazy<
  std::sync::Arc<rendiation_shadergraph::ShaderFunction>,
> = once_cell::sync::Lazy::new(|| {
  std::sync::Arc::new(rendiation_shadergraph::ShaderFunction::new("max", None))
});

pub fn max<T: ShaderGraphNodeType>(
  a: ShaderGraphNodeHandle<T>,
  b: ShaderGraphNodeHandle<T>,
) -> rendiation_shadergraph::ShaderGraphNodeHandle<T> {
  modify_graph(|graph| {
    let result = graph.nodes.create_node(
      ShaderGraphNode::<T>::new(ShaderGraphNodeData::Function(FunctionNode {
        prototype: MAX_FUNCTION.clone(),
      }))
      .to_any(),
    );
    unsafe {
      // #(#gen_node_connect)*
      result.cast_type()
    }
  })
}
