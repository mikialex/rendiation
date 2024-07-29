use crate::*;

#[derive(Default)]
pub struct DynamicTypeBuilder {
  state: Vec<(PrimitiveShaderValueType, PrimitiveShaderValue)>,
  node_to_resolve: Arc<RwLock<Option<NodeUntyped>>>,
}

impl DynamicTypeBuilder {
  fn bake(self) -> DynamicTypeBaked {
    todo!()
  }
}

pub struct DynamicTypeBaked {
  pub fields: Vec<(PrimitiveShaderValueType, PrimitiveShaderValue)>,
}

impl DynamicTypeBuilder {
  pub fn create_or_reconstruct_inline_state<T: PrimitiveShaderNodeType>(
    &mut self,
    default: T,
  ) -> BoxedShaderLoadStore<Node<T>> {
    let field_index = self.state.len();
    self.state.push((T::PRIMITIVE_TYPE, default.to_primitive()));

    let node = DeferResolvedStorageStructFieldNode {
      node: Arc::downgrade(&self.node_to_resolve),
      field_index: field_index as u32,
      resolved_node: Default::default(),
    };

    Box::new(node)
  }
}

struct DeferResolvedStorageStructFieldNode<T> {
  node: Weak<RwLock<Option<NodeUntyped>>>,
  field_index: u32,
  resolved_node: RwLock<Option<StorageNode<T>>>,
}
impl<T: PrimitiveShaderNodeType> ShaderAbstractLeftValue
  for DeferResolvedStorageStructFieldNode<T>
{
  type RightValue = Node<T>;
  fn abstract_load(&self) -> Node<T> {
    //  self.resolved_node.
    todo!()
  }

  fn abstract_store(&self, payload: Node<T>) {
    todo!()
  }
}
