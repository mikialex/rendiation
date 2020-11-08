use crate::{
  modify_graph, Node, OperatorNode, ShaderGraphConstableNodeType, ShaderGraphNode,
  ShaderGraphNodeData, ShaderGraphNodeType,
};
use std::ops::{Add, Mul, Sub};

impl<T> Add for Node<T>
where
  T: ShaderGraphNodeType + ShaderGraphConstableNodeType,
{
  type Output = Self;

  fn add(self, other: Self) -> Self {
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

impl<T> Sub for Node<T>
where
  T: ShaderGraphNodeType + ShaderGraphConstableNodeType,
{
  type Output = Self;

  fn sub(self, other: Self) -> Self {
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

impl<T> Mul for Node<T>
where
  T: ShaderGraphNodeType + ShaderGraphConstableNodeType,
{
  type Output = Self;

  fn mul(self, other: Self) -> Self {
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
