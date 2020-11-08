use rendiation_derives::glsl_function_inner;
use rendiation_shadergraph::*;

// macro_rules! impl_builtin_shader_fn {
//   (($($tt:ident)*), $name:ident) => {
//     pub fn max<T: ShaderGraphNodeType>(
//       a: Node<T>,
//       b: Node<T>,
//     ) -> Node<T> {
//       modify_graph(|graph| {
//         let node = ShaderGraphNode::<T>::new(ShaderGraphNodeData::Function(FunctionNode {
//           prototype: MAX_FUNCTION.clone(),
//         }));
//         let result = graph.insert_node(node).handle;
//         unsafe {
//           graph.nodes.connect_node(a.handle.cast_type(), result);
//           graph.nodes.connect_node(b.handle.cast_type(), result);
//           result.cast_type().into()
//         }
//       })
//     }
//   };
// }

pub fn length<T: ShaderGraphNodeType>(a: Node<T>) -> Node<f32> {
  modify_graph(|graph| {
    let node = ShaderGraphNode::<f32>::new(ShaderGraphNodeData::BuiltInFunction("length"));
    let result = graph.insert_node(node).handle;
    unsafe {
      graph.nodes.connect_node(a.handle.cast_type(), result);
      result.cast_type().into()
    }
  })
}

pub fn max<T: ShaderGraphNodeType>(a: Node<T>, b: Node<T>) -> Node<T> {
  modify_graph(|graph| {
    let node = ShaderGraphNode::<T>::new(ShaderGraphNodeData::BuiltInFunction("max"));
    let result = graph.insert_node(node).handle;
    unsafe {
      graph.nodes.connect_node(a.handle.cast_type(), result);
      graph.nodes.connect_node(b.handle.cast_type(), result);
      result.cast_type().into()
    }
  })
}

pub fn min<T: ShaderGraphNodeType>(a: Node<T>, b: Node<T>) -> Node<T> {
  modify_graph(|graph| {
    let node = ShaderGraphNode::<T>::new(ShaderGraphNodeData::BuiltInFunction("min"));
    let result = graph.insert_node(node).handle;
    unsafe {
      graph.nodes.connect_node(a.handle.cast_type(), result);
      graph.nodes.connect_node(b.handle.cast_type(), result);
      result.cast_type().into()
    }
  })
}

// could we do better?
glsl_function_inner!("vec4 vec4_31(vec3 a, float b){}///vec4");
glsl_function_inner!("vec4 vec4_13(float a, vec3 b){}///vec4");
glsl_function_inner!("vec4 vec4_22(vec2 a, vec2 b){}///vec4");

glsl_function_inner!("vec3 vec3_21(vec2 a, float b){}///vec3");
glsl_function_inner!("vec3 vec3_12(float a, vec2 b){}///vec3");
