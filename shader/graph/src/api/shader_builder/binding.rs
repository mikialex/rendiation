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
  pub bindings: Vec<ShaderGraphBindGroup>,
}

impl Default for ShaderGraphBindGroupBuilder {
  fn default() -> Self {
    Self {
      bindings: vec![Default::default(); 5],
    }
  }
}

pub struct UniformNodePreparer<T> {
  bindgroup_index: usize,
  entry_index: usize,
  phantom: PhantomData<T>,
  visibility_modifier: Rc<Cell<ShaderStageVisibility>>,
}

impl<T: PrimitiveShaderGraphNodeType> UniformNodePreparer<T> {
  pub fn get(&self) -> Node<T> {
    ShaderGraphInputNode::Uniform {
      bindgroup_index: self.bindgroup_index,
      entry_index: self.entry_index,
    }
    .insert_graph()
  }
}

impl ShaderGraphBindGroupBuilder {
  #[inline(never)]
  fn register_uniform_inner(&mut self, bindgroup_index: usize, ty: ShaderValueType) -> NodeUntyped {
    if let Ok(node) = self.query_uniform_inner(type_id, bindgroup_index) {
      return node;
    }

    let bindgroup = &mut self.bindings[bindgroup_index];

    let entry_index = bindgroup.bindings.len();

    bindgroup.bindings.push(ty);

    node
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
}
