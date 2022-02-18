use crate::*;

#[derive(Clone, Copy)]
pub enum SemanticBinding {
  Global,
  Camera,
  Pass,
  Material,
  Object,
}

/// simple and wonderful
pub type SB = SemanticBinding;

impl SemanticBinding {
  pub fn binding_index(&self) -> usize {
    match self {
      SemanticBinding::Global => 4,
      SemanticBinding::Camera => 3,
      SemanticBinding::Pass => 2,
      SemanticBinding::Material => 1,
      SemanticBinding::Object => 0,
    }
  }
}

impl From<SB> for usize {
  fn from(v: SB) -> Self {
    v.binding_index()
  }
}

/// should impl by user's container ty
pub trait ShaderUniformProvider: Any {
  type Node: ShaderGraphNodeType;
}

/// should impl by user's container ty
pub trait DynamicShaderUniformProvider: Any {
  fn to_value(&self) -> ShaderValueType;
}

pub struct ShaderGraphBindGroupBuilder {
  pub(crate) current_stage: ShaderStages,
  pub bindings: Vec<ShaderGraphBindGroup>,
}

impl Default for ShaderGraphBindGroupBuilder {
  fn default() -> Self {
    Self {
      current_stage: ShaderStages::Vertex,
      bindings: vec![Default::default(); 5],
    }
  }
}

impl ShaderGraphBindGroupBuilder {
  #[inline(never)]
  fn register_uniform_inner(
    &mut self,
    type_id: TypeId,
    bindgroup_index: usize,
    ty: ShaderValueType,
  ) -> NodeUntyped {
    if let Ok(node) = self.query_uniform_inner(type_id, bindgroup_index) {
      return node;
    }

    let bindgroup = &mut self.bindings[bindgroup_index];

    let entry_index = bindgroup.bindings.len();
    let node = ShaderGraphInputNode::Uniform {
      bindgroup_index,
      entry_index,
    }
    .insert_graph();

    let (node_vertex, node_fragment) = match self.current_stage {
      ShaderStages::Vertex => (node.handle().into(), None),
      ShaderStages::Fragment => (None, node.handle().into()),
    };

    bindgroup.bindings.push((
      ShaderGraphBindEntry {
        ty,
        node_vertex,
        node_fragment,
      },
      type_id,
    ));

    node
  }

  #[inline(never)]
  fn query_uniform_inner(
    &mut self,
    type_id: TypeId,
    bindgroup_index: usize,
  ) -> Result<NodeUntyped, ShaderGraphBuildError> {
    let current_stage = self.current_stage;
    let bindgroup = &mut self.bindings[bindgroup_index];

    bindgroup
      .bindings
      .iter_mut()
      .enumerate()
      .find(|(_, entry)| entry.1 == type_id)
      .map(|(i, (entry, _))| unsafe {
        let node = match current_stage {
          ShaderStages::Vertex => &mut entry.node_vertex,
          ShaderStages::Fragment => &mut entry.node_fragment,
        };
        node
          .get_or_insert_with(|| {
            ShaderGraphInputNode::Uniform {
              bindgroup_index,
              entry_index: i,
            }
            .insert_graph::<AnyType>()
            .handle()
          })
          .into_node()
      })
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  }

  #[inline]
  pub fn register_uniform_ty_inner<T: Any, N: ShaderGraphNodeType>(
    &mut self,
    index: impl Into<usize>,
  ) -> Node<N> {
    let node = self.register_uniform_inner(TypeId::of::<T>(), index.into(), N::to_type());
    unsafe { node.cast_type() }
  }

  #[inline]
  pub fn register_uniform<T: ShaderUniformProvider>(
    &mut self,
    index: impl Into<usize>,
  ) -> Node<T::Node> {
    self.register_uniform_ty_inner::<T, T::Node>(index)
  }

  #[inline]
  pub fn register_uniform_by<T: ShaderUniformProvider>(
    &mut self,
    _instance: &T,
    index: impl Into<usize>,
  ) -> Node<T::Node> {
    self.register_uniform::<T>(index)
  }

  /// N: the node type you want toc cast
  #[inline]
  pub fn register_uniform_dyn_ty_by<T, N>(
    &mut self,
    instance: &T,
    index: impl Into<usize>,
  ) -> Result<Node<N>, ShaderGraphBuildError>
  where
    T: DynamicShaderUniformProvider,
    N: ShaderGraphNodeType,
  {
    if instance.to_value() != N::to_type() {
      return Err(ShaderGraphBuildError::FailedDowncastShaderValueFromInput);
    }
    Ok(self.register_uniform_ty_inner::<T, N>(index))
  }

  #[inline]
  pub fn query_uniform<T: ShaderUniformProvider>(
    &mut self,
    index: impl Into<usize>,
  ) -> Result<Node<T::Node>, ShaderGraphBuildError> {
    let result = self.query_uniform_inner(TypeId::of::<T>(), index.into());
    result.map(|n| unsafe { n.cast_type() })
  }
}
