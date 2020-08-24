use rendiation_math::*;
use rendiation_shadergraph::*;

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
      graph.nodes.connect_node(a.cast_type(), result);
      graph.nodes.connect_node(b.cast_type(), result);
      result.cast_type()
    }
  })
}

// pub fn sampler2D(
//   texture: ShaderGraphNodeHandle<ShaderGraphTexture>,
//   sampler: ShaderGraphNodeHandle<ShaderGraphSampler>,
// ) -> ShaderGraphNodeHandle<ShaderGraphSampler> {
// }

pub static vec4_31_FUNCTION: once_cell::sync::Lazy<std::sync::Arc<ShaderFunction>> =
  once_cell::sync::Lazy::new(|| std::sync::Arc::new(ShaderFunction::new("max", None)));
pub fn vec4_31(
  a: ShaderGraphNodeHandle<Vec3<f32>>,
  b: ShaderGraphNodeHandle<f32>,
) -> ShaderGraphNodeHandle<Vec4<f32>> {
  modify_graph(|graph| {
    let result = graph.nodes.create_node(
      ShaderGraphNode::<Vec4<f32>>::new(ShaderGraphNodeData::Function(FunctionNode {
        prototype: vec4_31_FUNCTION.clone(),
      }))
      .to_any(),
    );
    unsafe {
      graph.nodes.connect_node(a.cast_type(), result);
      graph.nodes.connect_node(b.cast_type(), result);
      result.cast_type()
    }
  })
}
