use crate::{modify_graph, ShaderGraphNode, ShaderGraphNodeData, ShaderGraphNodeHandle};
use rendiation_math::{Vec3, Vec4};

impl ShaderGraphNodeHandle<Vec4<f32>> {
  pub fn xyz(&self) -> ShaderGraphNodeHandle<Vec3<f32>> {
    modify_graph(|graph| unsafe {
      let node = ShaderGraphNode::<Vec3<f32>>::new(ShaderGraphNodeData::Swizzle("xyz"));
      let result = graph.insert_node(node).handle;
      graph.nodes.connect_node(self.handle.cast_type(), result);
      result.cast_type().into()
    })
  }
}
