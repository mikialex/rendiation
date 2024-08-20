use crate::*;

pub struct DynamicTypeBuilder {
  meta: DynamicTypeMetaInfo,
  node_to_resolve: Arc<RwLock<Option<NodeUntyped>>>,
}

impl DynamicTypeBuilder {
  pub fn new_named(name: &str) -> Self {
    let mut v = Self {
      meta: DynamicTypeMetaInfo {
        ty: ShaderStructMetaInfo::new(name),
        fields_init: Default::default(),
      },
      node_to_resolve: Default::default(),
    };

    // make sure struct is not empty
    // todo, support empty
    v.create_or_reconstruct_inline_state_with_default::<u32>(0);
    v
  }
  pub fn meta_info(&self) -> DynamicTypeMetaInfo {
    self.meta.clone()
  }
  pub fn resolve(&mut self, node: NodeUntyped) {
    self.node_to_resolve.write().replace(node);
  }
}

#[derive(Clone)]
pub struct DynamicTypeMetaInfo {
  pub ty: ShaderStructMetaInfo,
  pub fields_init: Vec<Option<ShaderStructFieldInitValue>>,
}

impl DynamicTypeBuilder {
  pub fn create_or_reconstruct_inline_state_with_default<T: ShaderSizedValueNodeType>(
    &mut self,
    default: T,
  ) -> BoxedShaderLoadStore<Node<T>> {
    self.create_or_reconstruct_inline_state(Some(default))
  }

  pub fn create_or_reconstruct_inline_state<T: ShaderSizedValueNodeType>(
    &mut self,
    default: Option<T>,
  ) -> BoxedShaderLoadStore<Node<T>> {
    let field_index = self.meta.fields_init.len();
    self.meta.fields_init.push(default.map(|v| v.to_value()));
    self
      .meta
      .ty
      .push_field_dyn(&format!("field_{}", field_index), T::sized_ty());

    let node = DeferResolvedStorageStructFieldNode {
      node: Arc::downgrade(&self.node_to_resolve),
      field_index,
      resolved_node: Default::default(),
      ty: PhantomData,
    };

    Box::new(node)
  }

  pub fn create_or_reconstruct_any_left_value_by_right<T: ShaderAbstractRightValue>(
    &mut self,
  ) -> T::AbstractLeftValue {
    T::create_left_value_from_builder(self)
  }
}

impl LeftValueBuilder for DynamicTypeBuilder {
  fn create_single_left_value<T: ShaderSizedValueNodeType>(
    &mut self,
  ) -> BoxedShaderLoadStore<Node<T>> {
    self.create_or_reconstruct_inline_state(None)
  }
}

struct DeferResolvedStorageStructFieldNode<T> {
  node: Weak<RwLock<Option<NodeUntyped>>>,
  field_index: usize,
  resolved_node: RwLock<Option<NodeUntyped>>,
  ty: PhantomData<T>,
}

impl<T: ShaderSizedValueNodeType> DeferResolvedStorageStructFieldNode<T> {
  fn expect_resolved(&self) -> StorageNode<T> {
    let mut resolve = self.resolved_node.write();
    let storage = resolve.get_or_insert_with(|| {
      self
        .node
        .upgrade()
        .expect("dyn type builder lost")
        .read_recursive()
        .expect("dyn type builder not resolved yet")
    });

    unsafe { index_access_field(storage.handle(), self.field_index) }
  }
}
impl<T: ShaderSizedValueNodeType> ShaderAbstractLeftValue
  for DeferResolvedStorageStructFieldNode<T>
{
  type RightValue = Node<T>;
  fn abstract_load(&self) -> Node<T> {
    self.expect_resolved().load()
  }

  fn abstract_store(&self, payload: Node<T>) {
    self.expect_resolved().store(payload)
  }
}
