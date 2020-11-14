use crate::{
  modify_graph, Node, OperatorNode, ShaderGraphNode, ShaderGraphNodeData, ShaderGraphNodeType,
};
use std::ops::{Add, Mul, Sub};

impl<T, U> Add for Node<T>
where
  U: ShaderGraphNodeType,
  T: ShaderGraphNodeType + Add<Output = U>,
{
  type Output = Node<U>;

  fn add(self, other: Self) -> Self::Output {
    modify_graph(|graph| unsafe {
      let node = ShaderGraphNode::<T>::new(ShaderGraphNodeData::Operator(OperatorNode {
        left: self.handle.cast_type(),
        right: other.handle.cast_type(),
        operator: "+",
      }));
      let result = graph.insert_node(node).handle;
      graph.nodes.connect_node(self.handle.cast_type(), result);
      graph.nodes.connect_node(other.handle.cast_type(), result);
      result.cast_type().into()
    })
  }
}

impl<T, U> Sub for Node<T>
where
  U: ShaderGraphNodeType,
  T: ShaderGraphNodeType + Sub<Output = U>,
{
  type Output = Node<U>;

  fn sub(self, other: Self) -> Self::Output {
    modify_graph(|graph| unsafe {
      let node = ShaderGraphNode::<T>::new(ShaderGraphNodeData::Operator(OperatorNode {
        left: self.handle.cast_type(),
        right: other.handle.cast_type(),
        operator: "-",
      }));
      let result = graph.insert_node(node).handle;
      graph.nodes.connect_node(self.handle.cast_type(), result);
      graph.nodes.connect_node(other.handle.cast_type(), result);
      result.cast_type().into()
    })
  }
}

impl<T, U> Mul for Node<T>
where
  U: ShaderGraphNodeType,
  T: ShaderGraphNodeType + Mul<Output = U>,
{
  type Output = Node<U>;

  fn mul(self, other: Self) -> Self::Output {
    modify_graph(|graph| unsafe {
      let node = ShaderGraphNode::<T>::new(ShaderGraphNodeData::Operator(OperatorNode {
        left: self.handle.cast_type(),
        right: other.handle.cast_type(),
        operator: "*",
      }));
      let result = graph.insert_node(node).handle;
      graph.nodes.connect_node(self.handle.cast_type(), result);
      graph.nodes.connect_node(other.handle.cast_type(), result);
      result.cast_type().into()
    })
  }
}
