use crate::{
  Node, ShaderGraphNodeData, ShaderGraphNodeRawHandleUntyped, ShaderGraphNodeType,
  ShaderGraphStructuralNodeType,
};

impl<T> Node<T>
where
  T: ShaderGraphStructuralNodeType,
{
  pub fn expand(self) -> T::Instance {
    T::expand(self)
  }
}

pub fn expand_single<T>(
  struct_node: ShaderGraphNodeRawHandleUntyped,
  field_name: &'static str,
) -> Node<T>
where
  T: ShaderGraphNodeType,
{
  ShaderGraphNodeData::FieldGet {
    field_name,
    struct_node,
  }
  .insert_graph()
}

// pub fn construct_struct<T>(instance: T::Instance) -> Node<T>
// where
//   T: ShaderGraphStructuralNodeType,
// {
//   todo!()
// }
