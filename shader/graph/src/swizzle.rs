use crate::{modify_graph, Node, ShaderGraphNode, ShaderGraphNodeData, ShaderGraphNodeType};
use rendiation_algebra::{Vec3, Vec4};

fn swizzle_node<I: ShaderGraphNodeType, T: ShaderGraphNodeType>(
  n: &Node<I>,
  ty: &'static str,
) -> Node<T> {
  modify_graph(|graph| unsafe {
    let node = ShaderGraphNode::<Vec3<f32>>::new(ShaderGraphNodeData::Swizzle(ty));
    let result = graph.insert_node(node).handle;
    graph.nodes.connect_node(n.handle.cast_type(), result);
    result.cast_type().into()
  })
}

macro_rules! swizzle {
  ($IVec: ty, $OVec: ty, $Swi: tt) => {
    impl Node<$IVec> {
      pub fn xyz(&self) -> Node<$OVec> {
        swizzle_node::<_, _>(self, $Swi)
      }
    }
  };
}

swizzle!(Vec4<f32>, Vec3<f32>, "xyz");
