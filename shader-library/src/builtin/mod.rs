use rendiation_shadergraph::*;
use rendiation_shadergraph_derives::glsl_function_inner;

pub static MAX_FUNCTION: once_cell::sync::Lazy<std::sync::Arc<ShaderFunction>> =
  once_cell::sync::Lazy::new(|| std::sync::Arc::new(ShaderFunction::new("max", None)));

pub fn max<T: ShaderGraphNodeType>(
  a: ShaderGraphNodeHandle<T>,
  b: ShaderGraphNodeHandle<T>,
) -> ShaderGraphNodeHandle<T> {
  modify_graph(|graph| {
    let result = graph.nodes.create_node(
      ShaderGraphNode::<T>::new(ShaderGraphNodeData::Function(FunctionNode {
        prototype: MAX_FUNCTION.clone(),
      }))
      .to_any(),
    );
    unsafe {
      graph.nodes.connect_node(a.handle.cast_type(), result);
      graph.nodes.connect_node(b.handle.cast_type(), result);
      result.cast_type().into()
    }
  })
}

glsl_function_inner!("vec4 vec4_31(vec3 a, float b){}///vec4");
glsl_function_inner!("vec4 vec3_21(vec2 a, float b){}///vec4");
