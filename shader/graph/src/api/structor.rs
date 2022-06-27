use crate::*;

impl<T> Node<T>
where
  T: ShaderGraphStructuralNodeType,
{
  pub fn expand(self) -> T::Instance {
    T::expand(self)
  }
}

pub fn expand_single<T>(struct_node: ShaderGraphNodeRawHandle, field_name: &'static str) -> Node<T>
where
  T: ShaderGraphNodeType,
{
  ShaderGraphNodeExpr::FieldGet {
    field_name,
    struct_node,
  }
  .insert_graph()
}
