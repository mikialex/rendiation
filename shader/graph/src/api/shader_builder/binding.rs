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
pub trait ShaderUniformProvider {
  type Node: ShaderGraphNodeType;
}

/// should impl by user's container ty
pub trait DynamicShaderUniformProvider {
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

#[derive(Clone)]
pub struct UniformNodePreparer<T> {
  bindgroup_index: usize,
  entry_index: usize,
  phantom: PhantomData<T>,
  visibility_modifier: Rc<Cell<ShaderStageVisibility>>,
}

impl<T: ShaderGraphNodeType> UniformNodePreparer<T> {
  pub fn using(&self) -> Node<T> {
    match get_current_stage_unwrap() {
      ShaderStages::Vertex => match self.visibility_modifier.get() {
        ShaderStageVisibility::Fragment => {
          self.visibility_modifier.set(ShaderStageVisibility::Both)
        }
        ShaderStageVisibility::None => self.visibility_modifier.set(ShaderStageVisibility::Vertex),
        _ => {}
      },
      ShaderStages::Fragment => match self.visibility_modifier.get() {
        ShaderStageVisibility::Vertex => self.visibility_modifier.set(ShaderStageVisibility::Both),
        ShaderStageVisibility::None => self
          .visibility_modifier
          .set(ShaderStageVisibility::Fragment),
        _ => {}
      },
    }

    ShaderGraphInputNode::Uniform {
      bindgroup_index: self.bindgroup_index,
      entry_index: self.entry_index,
    }
    .insert_graph()
  }
}

impl ShaderGraphBindGroupBuilder {
  pub(crate) fn uniform_ty_inner<T, N: ShaderGraphNodeType>(
    &mut self,
    index: impl Into<usize>,
  ) -> UniformNodePreparer<N> {
    let bindgroup_index = index.into();
    let bindgroup = &mut self.bindings[bindgroup_index];

    let entry_index = bindgroup.bindings.len();
    let ty = N::TYPE;

    let visibility_modifier = Rc::new(Cell::new(ShaderStageVisibility::None));

    bindgroup.bindings.push((ty, visibility_modifier.clone()));

    UniformNodePreparer {
      bindgroup_index,
      entry_index,
      phantom: Default::default(),
      visibility_modifier,
    }
  }

  pub fn uniform<T: ShaderUniformProvider>(
    &mut self,
    index: impl Into<usize>,
  ) -> UniformNodePreparer<T::Node> {
    self.uniform_ty_inner::<T, T::Node>(index)
  }

  pub fn uniform_by<T: ShaderUniformProvider>(
    &mut self,
    _instance: &T,
    index: impl Into<usize>,
  ) -> UniformNodePreparer<T::Node> {
    self.uniform::<T>(index)
  }

  /// N: the node type you want toc cast
  pub fn uniform_dyn_ty_by<T, N>(
    &mut self,
    instance: &T,
    index: impl Into<usize>,
  ) -> Result<UniformNodePreparer<N>, ShaderGraphBuildError>
  where
    T: DynamicShaderUniformProvider,
    N: ShaderGraphNodeType,
  {
    if instance.to_value() != N::TYPE {
      return Err(ShaderGraphBuildError::FailedDowncastShaderValueFromInput);
    }
    Ok(self.uniform_ty_inner::<T, N>(index))
  }

  pub(crate) fn wrap(&mut self) -> ShaderGraphBindGroupDirectBuilder {
    ShaderGraphBindGroupDirectBuilder { builder: self }
  }
}

pub struct ShaderGraphBindGroupDirectBuilder<'a> {
  builder: &'a mut ShaderGraphBindGroupBuilder,
}

impl<'a> ShaderGraphBindGroupDirectBuilder<'a> {
  pub fn uniform<T: ShaderUniformProvider>(&mut self, index: impl Into<usize>) -> Node<T::Node> {
    self.builder.uniform_ty_inner::<T, T::Node>(index).using()
  }

  pub fn uniform_by<T: ShaderUniformProvider>(
    &mut self,
    _instance: &T,
    index: impl Into<usize>,
  ) -> Node<T::Node> {
    self.uniform::<T>(index)
  }

  /// N: the node type you want toc cast
  pub fn uniform_dyn_ty_by<T, N>(
    &mut self,
    instance: &T,
    index: impl Into<usize>,
  ) -> Result<Node<N>, ShaderGraphBuildError>
  where
    T: DynamicShaderUniformProvider,
    N: ShaderGraphNodeType,
  {
    if instance.to_value() != N::TYPE {
      return Err(ShaderGraphBuildError::FailedDowncastShaderValueFromInput);
    }
    Ok(self.builder.uniform_ty_inner::<T, N>(index).using())
  }
}
